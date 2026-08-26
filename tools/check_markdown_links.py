#!/usr/bin/env python3
"""Fail when a carried markdown link points at nothing.

What this defends
-----------------
Documentation is this project's authority surface: the boundary map, the owner
rulings, the policies, and the specs are all reached by relative link from
somewhere else. A dead link is not cosmetic — it is a reader arriving at
nothing where an authority was promised, and it survives review because the
prose around it still reads correctly.

Two dead deploy links did exactly that here: `deploy/production/README.md` and
`deploy/production/runbooks/deploy-rollback.md` pointed into a predecessor
documentation tree through two phases with every other check green (successor
issue #9).

Three assertions
----------------
1. **The target exists.** Every relative link in every carried markdown file
   resolves to a path inside the repository.
2. **The target is carried.** A link to a file git ignores is a link that
   works only on the author's machine. Directory targets are allowed; a
   directory is carried when it holds a carried file.
3. **The anchor exists.** A `#fragment` on a markdown target must match a
   heading in that file under GitHub's slug rule, or an explicit HTML anchor
   (`<a id="...">` / `name="..."`).

What it deliberately does not do
--------------------------------
No network. External `http(s)` and `mailto:` links are out of scope: a check
whose result depends on somebody else's uptime is a check that fails for
reasons the tree cannot fix. Reference-style link definitions are resolved;
links inside fenced code blocks are not links.

Fail-closed semantics
---------------------
An unreadable repository, or git being unavailable, exits 3. There is no
configuration file to lose, so there is no configuration to fail closed on.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

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

CHECK_NAME = "markdown-links"

#: Inline links: `[text](target)`. The target stops at the first `)` or space,
#: which is the shape every link in this tree uses.
INLINE_LINK = re.compile(r"\[[^\]]*\]\(\s*<?([^)<>\s]+)>?(?:\s+\"[^\"]*\")?\s*\)")
#: Reference definitions: `[label]: target`.
REFERENCE_DEFINITION = re.compile(r"^\s{0,3}\[[^\]]+\]:\s*<?([^\s<>]+)>?")
FENCE = re.compile(r"^\s*(```|~~~)")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
HTML_ANCHOR = re.compile(r"<a\s[^>]*(?:id|name)\s*=\s*[\"']([^\"']+)[\"']", re.IGNORECASE)
IGNORED_SCHEMES = ("http://", "https://", "mailto:", "ftp://", "tel:", "data:")


def slug(text: str) -> str:
    """GitHub's heading slug: strip formatting, lowercase, punctuation out."""
    text = re.sub(r"`([^`]*)`", r"\1", text)
    text = re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", text)
    text = re.sub(r"[*_]{1,3}([^*_]+)[*_]{1,3}", r"\1", text)
    text = text.strip().lower()
    text = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE)
    # GitHub replaces each space with a hyphen rather than collapsing runs, so
    # "observer / debug" (two spaces once the slash is stripped) becomes
    # "observer--debug". Collapsing here would reject links that work on the web.
    return text.replace(" ", "-")


def anchors(text: str) -> set[str]:
    """Every fragment the given markdown source offers.

    Duplicate headings get GitHub's `-1`, `-2` suffixes, so a link to the
    second "Rulings" heading resolves the way it does on the web.
    """
    found: set[str] = set()
    seen: dict[str, int] = {}
    in_fence = False
    for line in text.splitlines():
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        found.update(HTML_ANCHOR.findall(line))
        if in_fence:
            continue
        match = HEADING.match(line)
        if not match:
            continue
        base = slug(match.group(2))
        if not base:
            continue
        count = seen.get(base, 0)
        seen[base] = count + 1
        found.add(base if count == 0 else f"{base}-{count}")
    return found


def _link_targets(text: str) -> list[tuple[int, str]]:
    targets: list[tuple[int, str]] = []
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        # Inline code spans are not links either.
        stripped = re.sub(r"`[^`]*`", "", line)
        for target in INLINE_LINK.findall(stripped):
            targets.append((number, target))
        definition = REFERENCE_DEFINITION.match(stripped)
        if definition:
            targets.append((number, definition.group(1)))
    return targets


def scan(root: Path) -> list[Finding]:
    carried = carried_files(root)
    carried_set = set(carried)
    carried_directories = {
        parent
        for name in carried_set
        for parent in (str(Path(name).parent).replace("\\", "/"),)
        if parent not in (".", "")
    }
    # Every ancestor directory of a carried file is itself carried.
    for name in list(carried_directories):
        parts = name.split("/")
        for depth in range(1, len(parts)):
            carried_directories.add("/".join(parts[:depth]))

    anchor_cache: dict[str, set[str] | None] = {}
    findings: list[Finding] = []

    for relative in carried:
        if not relative.endswith(".md"):
            continue
        text = read_text(root / relative)
        if text is None:
            continue
        source = root / relative
        for number, raw_target in _link_targets(text):
            target = raw_target.strip()
            if not target or target.startswith(IGNORED_SCHEMES) or target.startswith("//"):
                continue
            path_part, _, fragment = target.partition("#")
            if path_part.startswith("/"):
                findings.append(
                    Finding(relative, f"absolute link target: {target}", line=number)
                )
                continue
            if path_part:
                resolved = (source.parent / path_part).resolve()
                try:
                    as_relative = resolved.relative_to(root.resolve()).as_posix()
                except ValueError:
                    findings.append(
                        Finding(relative, f"link escapes the repository: {target}", line=number)
                    )
                    continue
                if as_relative in carried_set:
                    pass
                elif as_relative in carried_directories:
                    continue
                elif resolved.exists():
                    findings.append(
                        Finding(
                            relative,
                            f"link target is not carried by git: {target}",
                            line=number,
                        )
                    )
                    continue
                else:
                    findings.append(
                        Finding(relative, f"link target does not exist: {target}", line=number)
                    )
                    continue
            else:
                as_relative = relative
                resolved = source
            if not fragment or not as_relative.endswith(".md"):
                continue
            if as_relative not in anchor_cache:
                target_text = read_text(root / as_relative)
                anchor_cache[as_relative] = None if target_text is None else anchors(target_text)
            available = anchor_cache[as_relative]
            if available is not None and fragment not in available:
                findings.append(
                    Finding(relative, f"no such anchor: {target}", line=number)
                )
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
