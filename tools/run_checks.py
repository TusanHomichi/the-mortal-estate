#!/usr/bin/env python3
"""Run every public-boundary check over the repository and summarize.

**This file owns the registry, not the spine.** `CHECKS` below is the single
place that says which public-boundary checks exist and where each one lives.
`tools/run_verification.py` reads this list to build one runner step per
check — it never restates the list — so a check that joins the registry joins
the verification runner in the same edit, and cannot be half-added.

Run directly, this is still the small thing that gives a human or a hook one
answer instead of five.

Exit codes: 0 when every check passes, 1 when any check reports violations,
3 when any check fails closed (a broken check outranks a dirty tree in the
summary, because you cannot trust the rest of the run).
"""

from __future__ import annotations

import argparse
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Sequence

sys.path.insert(0, str(Path(__file__).resolve().parent))

import check_boundary_terms  # noqa: E402
import check_clean_room  # noqa: E402
import check_hostnames  # noqa: E402
import check_markdown_links  # noqa: E402
import check_review_refs  # noqa: E402
from boundary_common import (  # noqa: E402
    EXIT_FAIL_CLOSED,
    EXIT_OK,
    EXIT_VIOLATION,
)

@dataclass(frozen=True)
class BoundaryCheck:
    """One registered check: its name, its script, and how to call it."""

    #: The name printed in the summary and used to derive a runner step key.
    name: str
    #: Repository-relative path to the script. The verification runner invokes
    #: the script rather than this module's function, so each check gets its
    #: own process, its own exit code, and its own timing.
    script: str
    #: In-process entry point, used when this file runs the checks itself.
    entry_point: Callable[[Sequence[str]], int]
    #: Whether `--terms` redirects this check's denylist.
    accepts_terms: bool


CHECKS: tuple[BoundaryCheck, ...] = (
    BoundaryCheck(
        "banned-terms", "tools/check_boundary_terms.py", check_boundary_terms.main, True
    ),
    BoundaryCheck("review-refs", "tools/check_review_refs.py", check_review_refs.main, False),
    BoundaryCheck("hostnames", "tools/check_hostnames.py", check_hostnames.main, False),
    BoundaryCheck("clean-room", "tools/check_clean_room.py", check_clean_room.main, False),
    BoundaryCheck(
        "markdown-links", "tools/check_markdown_links.py", check_markdown_links.main, False
    ),
)

_STATUS = {
    EXIT_OK: "PASS",
    EXIT_VIOLATION: "FAIL",
    EXIT_FAIL_CLOSED: "FAIL CLOSED",
}


def run(root: str | None = None, terms: str | None = None) -> int:
    base = [] if root is None else ["--root", root]
    results = []
    for check in CHECKS:
        arguments = list(base)
        if check.accepts_terms and terms is not None:
            arguments += ["--terms", terms]
        code = check.entry_point(arguments)
        results.append((check.name, code))

    print("")
    print("boundary checks")
    print("---------------")
    for name, code in results:
        print(f"  {_STATUS.get(code, f'EXIT {code}'):<11} {name}")

    codes = [code for _, code in results]
    if EXIT_FAIL_CLOSED in codes:
        summary, exit_code = "one or more checks FAILED CLOSED", EXIT_FAIL_CLOSED
    elif any(code != EXIT_OK for code in codes):
        summary, exit_code = "violations found", EXIT_VIOLATION
    else:
        summary, exit_code = "all checks passed", EXIT_OK
    print(f"\n{summary}")
    return exit_code


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", default=None, help="repository root to scan")
    parser.add_argument(
        "--terms",
        default=None,
        help=(
            "denylist file for the banned-terms check; omitted, that check uses "
            "its own default (the private .boundary/ list). CI passes the tracked "
            "synthetic fixture so the mechanism runs where the real list cannot."
        ),
    )
    arguments = parser.parse_args(argv)
    return run(arguments.root, arguments.terms)


if __name__ == "__main__":
    sys.exit(main())
