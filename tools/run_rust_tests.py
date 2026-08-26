#!/usr/bin/env python3
"""Run the Rust workspace's tests through the bounded scheduler.

The scheduler itself is `tools/verification/rust_tests.py`; this is its command
line, so the verification step table can name one script and a human can run
the same thing directly.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from verification.rust_tests import DEFAULT_JOBS, execute  # noqa: E402


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--jobs",
        type=int,
        default=DEFAULT_JOBS,
        help=f"test executables to run at a time (default: {DEFAULT_JOBS})",
    )
    arguments = parser.parse_args(argv)
    if arguments.jobs < 1:
        parser.error("--jobs must be positive")
    return execute(jobs=arguments.jobs)


if __name__ == "__main__":
    raise SystemExit(main())
