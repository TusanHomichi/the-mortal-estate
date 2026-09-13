"""Inspect index and working-tree whitespace independently.

A clean working tree can conceal a staged defect, and an unstaged correction
can cancel it in a combined HEAD diff. Both pending surfaces must be checked.
"""

from __future__ import annotations

import subprocess


def main() -> int:
    results = [
        subprocess.run(["git", "diff", "--check"], check=False).returncode,
        subprocess.run(["git", "diff", "--cached", "--check"], check=False).returncode,
    ]
    return next((code for code in results if code != 0), 0)


if __name__ == "__main__":
    raise SystemExit(main())
