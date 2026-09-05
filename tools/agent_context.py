#!/usr/bin/env python3
"""Route a task to the documents that own it.

    python3 tools/agent_context.py --list
    python3 tools/agent_context.py --path crates/tme-server/src/store/mod.rs
    python3 tools/agent_context.py --subject workbench-v0
    python3 tools/agent_context.py --validate

**The routing is derived, not restated.** Every document under `docs/` declares
in its own front matter what it owns — `routes:` for the paths it speaks for,
`always: true` for standing guidance that applies to every task.
Read the relevant sections; the marker does not require loading full documents. This tool reads that and answers;
`--validate` asserts the "Read first" table in `AGENTS.md` agrees with it. A
hand-maintained routing table drifts the first time somebody adds a document and
forgets a row, and the drift is invisible because the table still reads
correctly.

Three things it deliberately is not:

* **Not a substitute for reading.** It locates authority; it never summarises it.
* **Not generated documentation.** It writes nothing. `AGENTS.md` stays
  hand-written prose and this checks it, which is the opposite direction.
* **Not fuzzy.** A path that matches no document is reported as matching no
  document, not as matching the closest thing.
"""

from __future__ import annotations

import argparse
import fnmatch
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
CONTRACT = "AGENTS.md"

#: Front-matter keys every routed document must carry. `summary` is what this
#: tool prints, so a document without one cannot be routed to usefully.
REQUIRED_KEYS = ("summary", "status", "last_updated")


class RoutingError(RuntimeError):
    """The routing cannot be resolved as specified. Always fails closed."""


@dataclass(frozen=True)
class Document:
    subject: str
    path: str
    summary: str
    routes: tuple[str, ...]
    always: bool

    def matches(self, path: str) -> bool:
        return any(_glob(path, pattern) for pattern in self.routes)

    def specificity(self, path: str) -> int:
        """How narrow the matching route is. A longer literal prefix wins."""
        return max(
            (len(pattern.split("*")[0]) for pattern in self.routes if _glob(path, pattern)),
            default=0,
        )


def _glob(path: str, pattern: str) -> bool:
    """`**` crosses directory separators; `*` does not."""
    if "**" in pattern:
        head, _, tail = pattern.partition("**")
        if not path.startswith(head):
            return False
        return not tail or fnmatch.fnmatch(path, f"*{tail}") or path.endswith(tail.strip("/"))
    return fnmatch.fnmatch(path, pattern)


def read_front_matter(text: str) -> dict[str, object] | None:
    """Parse the small YAML subset these documents use.

    Scalars and block lists, nothing else. A real YAML parser is not in the
    standard library and this runner is standard library only; a subset that
    refuses what it does not understand is safer than a dependency here.
    """
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    try:
        end = lines.index("---", 1)
    except ValueError as error:
        raise RoutingError("front matter is opened and never closed") from error
    parsed: dict[str, object] = {}
    key: str | None = None
    for line in lines[1:end]:
        if not line.strip():
            continue
        if line.startswith(("  - ", "- ")):
            if key is None:
                raise RoutingError(f"list entry before any key: {line!r}")
            parsed.setdefault(key, [])
            entry = line.split("- ", 1)[1].strip().strip("\"'")
            if not isinstance(parsed[key], list):
                raise RoutingError(f"{key} is both a scalar and a list")
            parsed[key].append(entry)  # type: ignore[union-attr]
            continue
        if ":" not in line:
            raise RoutingError(f"front-matter line is neither a key nor a list entry: {line!r}")
        key, _, value = line.partition(":")
        key = key.strip()
        value = value.strip()
        if value:
            parsed[key] = value.strip("\"'")
    return parsed


def load(root: Path = ROOT) -> tuple[Document, ...]:
    documents: list[Document] = []
    for path in sorted((root / "docs").glob("*.md")):
        relative = path.relative_to(root).as_posix()
        front = read_front_matter(path.read_text(encoding="utf-8"))
        if front is None:
            raise RoutingError(f"{relative} carries no front matter and cannot be routed")
        missing = [key for key in REQUIRED_KEYS if key not in front]
        if missing:
            raise RoutingError(f"{relative} front matter lacks {missing}")
        routes = front.get("routes", [])
        if isinstance(routes, str):
            raise RoutingError(f"{relative}: routes must be a list, not a scalar")
        documents.append(
            Document(
                subject=path.stem,
                path=relative,
                summary=str(front["summary"]),
                routes=tuple(routes),
                always=str(front.get("always", "false")).lower() == "true",
            )
        )
    subjects = [document.subject for document in documents]
    duplicates = sorted({name for name in subjects if subjects.count(name) > 1})
    if duplicates:
        raise RoutingError(f"duplicate subjects: {duplicates}")
    return tuple(documents)


def read_first(documents: tuple[Document, ...], path: str) -> tuple[Document, ...]:
    """The documents that own a path, most specific first, always-read last."""
    matched = sorted(
        (document for document in documents if document.matches(path)),
        key=lambda document: (-document.specificity(path), document.subject),
    )
    standing = [document for document in documents if document.always]
    return (*matched, *(item for item in standing if item not in matched))


def contract_rows(root: Path = ROOT) -> set[str]:
    """Every `docs/...` path the contract's Read-first table names."""
    text = (root / CONTRACT).read_text(encoding="utf-8")
    start = text.index("## Read first")
    end = text.index("\n## ", start + 1)
    rows: set[str] = set()
    for line in text[start:end].splitlines():
        if not line.startswith("| ["):
            continue
        target = line.split("](", 1)[1].split(")", 1)[0]
        if target.startswith("docs/"):
            rows.add(target)
    return rows


def validate(root: Path = ROOT) -> list[str]:
    """Every way the routing can be wrong, as one-line diagnostics."""
    problems: list[str] = []
    documents = load(root)
    for document in documents:
        for pattern in document.routes:
            head = pattern.split("*")[0].rstrip("/")
            if head and not (root / head).exists() and not list(root.glob(pattern)):
                problems.append(f"{document.path}: route {pattern!r} matches nothing in this tree")
        if not document.routes and not document.always:
            problems.append(
                f"{document.path}: declares neither routes nor always; no task will be sent to it"
            )
    routed = {document.path for document in documents}
    listed = contract_rows(root)
    for missing in sorted(routed - listed):
        problems.append(f"{CONTRACT} 'Read first' does not list {missing}")
    for stale in sorted(listed - routed):
        problems.append(f"{CONTRACT} 'Read first' lists {stale}, which docs/ does not carry")
    return problems


def _print(documents: tuple[Document, ...]) -> None:
    for document in documents:
        marker = "always" if document.always else ", ".join(document.routes) or "-"
        print(f"{document.subject}\n  {document.path}\n  routes: {marker}\n  {document.summary}\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument("--list", action="store_true", help="every routable document")
    action.add_argument("--path", metavar="PATH", help="what to read before changing this path")
    action.add_argument("--subject", metavar="SLUG", help="one document by its file stem")
    action.add_argument(
        "--validate", action="store_true", help="assert the routing and the contract agree"
    )
    arguments = parser.parse_args(argv)
    try:
        if arguments.validate:
            problems = validate()
            if problems:
                print(f"routing: {len(problems)} problem(s)")
                for problem in problems:
                    print(f"  {problem}")
                return 1
            print("routing: OK")
            return 0
        documents = load()
        if arguments.list:
            _print(documents)
            return 0
        if arguments.subject:
            chosen = [item for item in documents if item.subject == arguments.subject]
            if not chosen:
                print(f"no document with subject {arguments.subject!r}", file=sys.stderr)
                return 1
            _print(tuple(chosen))
            return 0
        selected = read_first(documents, arguments.path)
        print(f"read first, before changing {arguments.path}:\n")
        _print(selected)
        return 0
    except (RoutingError, OSError, ValueError) as error:
        print(f"routing: FAIL CLOSED — {error}", file=sys.stderr)
        return 3


if __name__ == "__main__":
    sys.exit(main())
