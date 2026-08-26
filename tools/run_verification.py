#!/usr/bin/env python3
"""The Mortal Estate's verification runner — the single source of truth.

This file is deliberately thin. The spine lives in `tools/verification/`, split
by responsibility so no piece of it grows past the point where it is read. Ask
it what it will do before asking it to do anything:

    python3 tools/run_verification.py --list --scope full

Documentation names this command rather than restating the commands it prints.
A hand-copied command list is a second source of truth, and a second source of
truth is a drift you have not noticed yet.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from verification.cli import main  # noqa: E402

if __name__ == "__main__":
    raise SystemExit(main())
