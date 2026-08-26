#!/usr/bin/env python3
"""Apply the working root's retention ruling to Workbench sessions.

    tools/workbench_prune.py            # remove what the ruling says to remove
    tools/workbench_prune.py --dry-run  # say what it would remove
    tools/workbench_prune.py --keep <session-id>

The ruling is in [docs/working-root-policy.md](../docs/working-root-policy.md):
sessions older than fourteen days, and any session beyond the most recent ten,
are removed. Nothing tracked references a session, so this can never break a
build, a test, a proof, or a promotion — which is the property that makes an
automatic cleanup safe to run without asking.

The decision lives in `tools/workbench/session.py` next to the sessions it is
about; this is its command line.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))

from workbench import session as workbench_session  # noqa: E402


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--keep", default=None, help="a session id to spare, whatever its age")
    parser.add_argument("--dry-run", action="store_true", help="report without removing")
    parser.add_argument(
        "--days",
        type=int,
        default=workbench_session.RETENTION_DAYS,
        help=f"age in days beyond which a session goes (default: {workbench_session.RETENTION_DAYS})",
    )
    arguments = parser.parse_args(argv)
    if arguments.dry_run:
        doomed = workbench_session.prunable(ROOT, keep=arguments.keep, days=arguments.days)
        verb = "would remove"
    else:
        doomed = workbench_session.prune(ROOT, keep=arguments.keep, days=arguments.days)
        verb = "removed"
    for path in doomed:
        print(f"{verb} {path.relative_to(ROOT)}")
    print(f"{verb} {len(doomed)} session(s); retention is docs/working-root-policy.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
