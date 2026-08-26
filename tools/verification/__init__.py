"""The verification spine.

`tools/run_verification.py` is the entry point; everything it does lives here:

* `model` — steps, capabilities, outcomes, the verdict and its exit codes
* `capabilities` — what the environment provides, probed once, with reasons
* `table` — the step table and the scope partition
* `resolve` — scopes and changed paths into an ordered list of steps
* `targets` — proof that every target the table names exists
* `rust_tests` — the bounded Rust test scheduler
* `footprint` — what a build costs on disk, and the disposable-build profile
* `execute` — running, timing, and the honest verdict
* `cli` — argument parsing and `--list`

Split this way for the 1,000-line rule and because each piece is separately
testable: `tests/test_verification_*.py` covers them one module at a time.
"""

from __future__ import annotations

__all__ = [
    "capabilities",
    "cli",
    "execute",
    "footprint",
    "model",
    "resolve",
    "rust_tests",
    "table",
    "targets",
]
