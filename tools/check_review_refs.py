#!/usr/bin/env python3
"""Fail when a content provenance reference does not resolve against the tree.

What this defends
-----------------
Content files declare their provenance in a `research_boundary` object whose
`review_refs` array names the records that authorized the content. In the
predecessor project those entries were free text and the validator only
required them to be non-empty, so dropping a documentation root orphaned the
provenance chain of the entire clean corpus **without failing any check** — a
textbook false green.

This check resolves every entry as a path against the repository. It is built
before any content exists on purpose: with an empty tree it is trivially green,
which is the correct result, and it means the first orphaned reference fails on
arrival rather than being discovered six phases later.

What is scanned
---------------
Every carried `.json` file. `research_boundary.review_refs` is found at any
nesting depth, so a content schema may move the object without escaping the
check. A carried `.json` file that does not parse is a violation: a file whose
provenance cannot be read cannot be proven clean.

Resolution rule
---------------
An entry must be a non-empty string naming a repository-relative path. A
trailing `#fragment` (a section anchor) is stripped before resolving. The
resolved path must exist inside the repository and must be something the
repository actually carries: a carried file, or a directory containing at least
one carried file. Absolute paths, and paths that escape the repository root,
are violations — a provenance chain that points outside the tree is not a
chain.

Fail-closed semantics
---------------------
This check has no external configuration to lose, so its only fail-closed cases
are structural: an unresolvable root or an unavailable git exits 3.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path, PurePosixPath

sys.path.insert(0, str(Path(__file__).resolve().parent))

from boundary_common import (  # noqa: E402
    ConfigError,
    Finding,
    carried_files,
    fail_closed,
    read_text,
    report,
    resolve_root,
)

CHECK_NAME = "review-refs"
BOUNDARY_KEY = "research_boundary"
REFS_KEY = "review_refs"


def find_review_ref_arrays(node, pointer: str = "") -> list[tuple[str, object]]:
    """Return every (json pointer, value) pair for research_boundary.review_refs."""
    found: list[tuple[str, object]] = []
    if isinstance(node, dict):
        for key, value in node.items():
            child = f"{pointer}/{key}"
            if key == BOUNDARY_KEY and isinstance(value, dict) and REFS_KEY in value:
                found.append((f"{child}/{REFS_KEY}", value[REFS_KEY]))
            found.extend(find_review_ref_arrays(value, child))
    elif isinstance(node, list):
        for index, value in enumerate(node):
            found.extend(find_review_ref_arrays(value, f"{pointer}/{index}"))
    return found


def _resolves(root: Path, carried: set[str], entry: str) -> str | None:
    """Return None when the entry resolves, or the reason it does not."""
    target = entry.split("#", 1)[0].strip()
    if not target:
        return "entry is empty"
    posix = PurePosixPath(target)
    if not posix.parts:
        return f"entry does not name a path: {entry!r}"
    if posix.is_absolute() or target.startswith("\\") or ":" in posix.parts[0]:
        return f"entry is not a repository-relative path: {entry!r}"
    normalized = PurePosixPath(*[part for part in posix.parts if part != "."])
    if ".." in normalized.parts:
        return f"entry escapes the repository root: {entry!r}"
    relative = normalized.as_posix()
    if relative in carried:
        return None
    prefix = relative.rstrip("/") + "/"
    if any(name.startswith(prefix) for name in carried):
        return None
    return f"entry does not resolve against the tree: {entry!r}"


def scan(root: Path) -> list[Finding]:
    carried = carried_files(root)
    carried_set = set(carried)
    findings: list[Finding] = []

    for relative in carried:
        if not relative.endswith(".json"):
            continue
        text = read_text(root / relative)
        if text is None:
            findings.append(Finding(relative, "carried .json file is not readable text"))
            continue
        try:
            document = json.loads(text)
        except json.JSONDecodeError as error:
            findings.append(
                Finding(relative, f"carried .json file does not parse: {error}")
            )
            continue
        for pointer, value in find_review_ref_arrays(document):
            if not isinstance(value, list):
                findings.append(Finding(relative, f"{pointer} is not an array"))
                continue
            if not value:
                findings.append(Finding(relative, f"{pointer} is empty"))
                continue
            for index, entry in enumerate(value):
                location = f"{pointer}/{index}"
                if not isinstance(entry, str):
                    findings.append(
                        Finding(relative, f"{location} is not a string: {entry!r}")
                    )
                    continue
                reason = _resolves(root, carried_set, entry)
                if reason:
                    findings.append(Finding(relative, f"{location} {reason}"))
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root to scan")
    arguments = parser.parse_args(argv)

    try:
        findings = scan(resolve_root(arguments.root))
    except ConfigError as error:
        return fail_closed(CHECK_NAME, error)
    return report(CHECK_NAME, findings)


if __name__ == "__main__":
    sys.exit(main())
