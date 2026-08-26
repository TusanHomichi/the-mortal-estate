#!/usr/bin/env python3
"""Fail when the tree depends on, or could commit, a predecessor-private root.

What this defends
-----------------
This project is clean-room. The predecessor kept its source research under one
private root and its non-shipping runtime fixtures under another, both
git-ignored there. Neither exists here, and the genesis plan requires proving
that the tree builds and tests with them **absent from the filesystem
entirely** — not merely unreferenced. A tree that quietly reads a path only its
author has is a tree that cannot be handed to anyone.

Three assertions
----------------
1. **No load-bearing reference.** The private-root path strings may appear only
   in files named by the tracked doc allowlist. Documentation *about* the
   policy has to name the roots to state the policy; code and content may not
   name them at all. The allowlist is short, tracked, and every entry carries
   its reason.
2. **The roots are absent.** Neither private root exists on disk at the
   repository root.
3. **The ignored roots are genuinely ignored.** `git check-ignore` confirms
   that each private root, and the private boundary-data root, cannot be
   committed by accident. This is what keeps the banned-term data file private
   in practice rather than by good intentions.

Fail-closed semantics
---------------------
A missing, unreadable, or entry-less allowlist file exits 3, as does an
unavailable git. An allowlist entry naming a file the tree does not carry is a
violation, not a shrug: a stale exemption is an exemption nobody is watching.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from boundary_common import (  # noqa: E402
    ConfigError,
    Finding,
    carried_files,
    fail_closed,
    git_is_ignored,
    load_list_file,
    read_text,
    report,
    resolve_root,
)

CHECK_NAME = "clean-room"
DEFAULT_ALLOWLIST_PATH = "tools/clean-room-allowlist.txt"

# Predecessor-private roots. A carried file may name these only if the doc
# allowlist exempts it; the roots themselves must not exist on disk.
PRIVATE_ROOTS = ("Research/", "placeholders/")

# Roots that must be uncommittable. The private roots plus this project's own
# private boundary-data root.
MUST_BE_IGNORED = ("Research/", "placeholders/", ".boundary/")


def scan(root: Path, allowlist_path: Path) -> list[Finding]:
    allowed = load_list_file(allowlist_path, "clean-room-allowlist")
    allowed_set = set(allowed)
    carried = carried_files(root)
    carried_set = set(carried)
    findings: list[Finding] = []

    try:
        allowlist_relative = allowlist_path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        allowlist_relative = None

    for entry in sorted(allowed_set):
        if entry not in carried_set:
            findings.append(
                Finding(
                    allowlist_relative or str(allowlist_path),
                    f"allowlist exempts a file the tree does not carry: {entry!r}",
                )
            )

    for relative in carried:
        if relative in allowed_set or relative == allowlist_relative:
            continue
        text = read_text(root / relative)
        if text is None:
            continue
        for number, line in enumerate(text.splitlines(), start=1):
            for private_root in PRIVATE_ROOTS:
                if private_root in line:
                    findings.append(
                        Finding(
                            relative,
                            f"references private root {private_root!r} "
                            "and is not on the clean-room doc allowlist",
                            line=number,
                        )
                    )

    for private_root in PRIVATE_ROOTS:
        if (root / private_root.rstrip("/")).exists():
            findings.append(
                Finding(
                    private_root,
                    "private root exists on disk; the clean-room proof requires "
                    "it to be absent entirely, not merely unreferenced",
                )
            )

    for ignored_root in MUST_BE_IGNORED:
        probe = f"{ignored_root.rstrip('/')}/probe"
        if not git_is_ignored(root, probe):
            findings.append(
                Finding(
                    ".gitignore",
                    f"{ignored_root!r} is not ignored; a private file placed there "
                    "could be committed by accident",
                )
            )
    return findings


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root to scan")
    parser.add_argument(
        "--allowlist",
        default=None,
        help=f"doc allowlist data file (default: <root>/{DEFAULT_ALLOWLIST_PATH})",
    )
    arguments = parser.parse_args(argv)

    try:
        root = resolve_root(arguments.root)
        allowlist_path = (
            Path(arguments.allowlist).resolve()
            if arguments.allowlist
            else root / DEFAULT_ALLOWLIST_PATH
        )
        findings = scan(root, allowlist_path)
    except ConfigError as error:
        return fail_closed(CHECK_NAME, error)
    return report(CHECK_NAME, findings)


if __name__ == "__main__":
    sys.exit(main())
