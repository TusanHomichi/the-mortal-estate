"""Argument parsing, `--list`, and the exit-code contract.

    python3 tools/run_verification.py --list --scope full
    python3 tools/run_verification.py --scope fast --changed-path docs/boundary-map.md
    python3 tools/run_verification.py --scope full --allow-unavailable

Exit codes:

* **0** — every selected step ran and passed (or `--allow-unavailable` was
  given and nothing failed).
* **1** — a step failed.
* **2** — usage error.
* **3** — INCOMPLETE: nothing failed, but a step could not run or ran degraded.
"""

from __future__ import annotations

import argparse
import os
import sys
from typing import Mapping, Sequence

from . import footprint, targets
from .capabilities import BY_NAME, evaluate_all
from .model import EXIT_INCOMPLETE, EXIT_OK, EXIT_USAGE, ResolutionError
from .execute import execute, print_verdict
from .resolve import Selection, select_changed_paths, steps_for
from .table import ALL_SCOPES, COMPOSED, OUT_OF_BASELINE, OWNER_SCOPES, ROOT, STEPS


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--scope",
        action="append",
        choices=ALL_SCOPES,
        help="lane to run; repeat to compose. `fast` requires --changed-path.",
    )
    parser.add_argument(
        "--changed-path",
        action="append",
        default=[],
        metavar="PATH",
        help="repository-relative changed path; repeat. Resolves the fast lane.",
    )
    parser.add_argument(
        "--list",
        action="store_true",
        help="print the resolved commands and stop, running nothing",
    )
    parser.add_argument(
        "--allow-unavailable",
        action="store_true",
        help="exit 0 rather than 3 when a capability this caller cannot supply is absent",
    )
    parser.add_argument(
        "--keep-going",
        action="store_true",
        help="run every selected step even after one fails",
    )
    parser.add_argument(
        "--report-disk",
        action="store_true",
        help=(
            "print the cargo target directory's size and its filesystem's free "
            "space after every step that builds; how a disk-exhausted run says so"
        ),
    )
    parser.add_argument(
        "--check-step-targets",
        action="store_true",
        help="verify every target the step table names exists, and stop",
    )
    parser.add_argument(
        "--capabilities",
        action="store_true",
        help="probe every capability, print what is available, and stop",
    )
    return parser


def _resolve(arguments: argparse.Namespace) -> Selection:
    scopes = tuple(arguments.scope or ())
    changed = tuple(arguments.changed_path)
    if "fast" in scopes:
        if len(scopes) > 1:
            raise ResolutionError("--scope fast cannot be combined with another scope")
        return select_changed_paths(changed)
    if changed and scopes:
        raise ResolutionError(
            "--changed-path resolves the lane itself; do not also name a scope "
            "(use --scope fast --changed-path ... to be explicit)"
        )
    if changed:
        return select_changed_paths(changed)
    if not scopes:
        raise ResolutionError("give --scope, or --changed-path, or --scope fast --changed-path")
    for scope in scopes:
        if scope in OUT_OF_BASELINE:
            # Requesting it explicitly is exactly how an owner invokes it.
            continue
    return Selection(scopes, (f"explicit scope selection: {', '.join(scopes)}",))


def _print_list(selection: Selection, steps, environ: Mapping[str, str]) -> None:
    states = evaluate_all(environ)
    for reason in selection.reasons:
        print(f"reason :: {reason}")
    print(f"scopes :: {', '.join(selection.scopes)}")
    print("")
    selected = {step.key for step in steps}
    for step in steps:
        missing = [name for name in step.requires if not states[name].available]
        degraded = step.degrades_without and not states[step.degrades_without].available
        argv = list(step.degraded_argv or step.argv) if degraded else list(step.argv)
        marker = "UNAVAILABLE" if missing else ("DEGRADED   " if degraded else "SELECT     ")
        print(f"{marker} {step.key:<28} {' '.join(argv)}")
        if missing:
            print(f"{'':<12} {'':<28} :: {'; '.join(states[name].reason for name in missing)}")
    for key, step in STEPS.items():
        if key not in selected:
            note = "owner-invoked" if step.owner in OUT_OF_BASELINE else "not selected"
            print(f"SKIP        {key:<28} :: {note}")


def main(argv: Sequence[str] | None = None, environ: Mapping[str, str] | None = None) -> int:
    parser = build_parser()
    arguments = parser.parse_args(argv)
    environ = os.environ if environ is None else environ

    if arguments.check_step_targets:
        return targets.report(STEPS)

    if arguments.capabilities:
        for name, state in sorted(evaluate_all(environ).items()):
            print(f"{'AVAILABLE' if state.available else 'ABSENT':<10} {name:<16} {state.reason}")
        return EXIT_OK

    try:
        selection = _resolve(arguments)
        steps = steps_for(selection.scopes)
    except ResolutionError as error:
        print(f"FAIL resolution: {error}", file=sys.stderr)
        return EXIT_USAGE

    if arguments.list:
        _print_list(selection, steps, environ)
        return EXIT_OK

    for reason in selection.reasons:
        print(f"reason :: {reason}")
    print(f"scopes :: {', '.join(selection.scopes)}\n")
    if arguments.report_disk:
        print(footprint.describe(footprint.target_directory(environ, ROOT)))
    verdict = execute(
        steps,
        environ=environ,
        keep_going=arguments.keep_going,
        report_disk=arguments.report_disk,
    )
    print_verdict(verdict, allow_unavailable=arguments.allow_unavailable)
    return verdict.exit_code(allow_unavailable=arguments.allow_unavailable)


#: Re-exported so tests can assert the composition without importing the table.
__all__ = ["main", "build_parser", "COMPOSED", "OWNER_SCOPES", "EXIT_INCOMPLETE"]
