"""Shared scaffolding for the verification-spine tests.

The spine lives under `tools/`, which is not a package, so this puts `tools/`
on the path once and every suite imports `verification.*` from there — the same
convention `boundary_test_support` uses for the boundary checks.
"""

from __future__ import annotations

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
TOOLS = REPO_ROOT / "tools"

if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))
