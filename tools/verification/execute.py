"""Run resolved steps, time them, and reach an honest verdict.

The one contract worth reading twice: **UNAVAILABLE is never PASS.** A step
whose capability is absent does not run, is reported as unavailable with the
reason, and makes the whole run INCOMPLETE. An incomplete run exits 3 — the
same code the boundary checks use for "could not run as specified" — unless
the caller passes `--allow-unavailable`, which is how a lane that genuinely
cannot provide a capability (CI has no display, no database, and no private
denylist) says so out loud instead of quietly counting a skip as proof.
"""

from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Callable, Mapping, Sequence

from . import footprint
from .capabilities import evaluate_all
from .model import (
    EnvironmentExpansionError,
    Step,
    StepOutcome,
    Verdict,
    expand_argv,
)
from .table import CARGO_OWNERS, ROOT

#: Modes whose output is captured and summarised rather than streamed.
_CAPTURED = frozenset({"quiet_unittest", "empty_stdout"})


def _summarise_unittest(stderr: str) -> str:
    lines = [
        line
        for line in stderr.splitlines()
        if line.startswith("Ran ") or line in {"OK"} or line.startswith("OK (")
    ]
    return " ".join(lines) if lines else "completed"


def run_step(
    step: Step,
    argv: Sequence[str],
    *,
    root: Path = ROOT,
    environ: Mapping[str, str],
    runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
) -> tuple[bool, str]:
    captured = step.mode in _CAPTURED
    environment = dict(environ)
    if step.mode == "quiet_unittest":
        # `unittest discover -s tests` puts tests/ on the path implicitly; the
        # named-module form does not, and every suite here imports its support
        # module by bare name. Supplying the same path keeps one import
        # convention across both ways of running the suite.
        existing = environment.get("PYTHONPATH", "")
        environment["PYTHONPATH"] = os.pathsep.join(
            [str(root / "tests"), *( [existing] if existing else [] )]
        )
    try:
        completed = runner(
            list(argv),
            cwd=str(root),
            env=environment,
            capture_output=captured,
            text=True,
            timeout=step.timeout,
            check=False,
        )
    except subprocess.TimeoutExpired:
        return False, f"exceeded its {step.timeout:.0f}s ceiling"
    except OSError as error:
        return False, str(error)
    if completed.returncode != 0:
        if captured:
            sys.stdout.write(completed.stdout or "")
            sys.stderr.write(completed.stderr or "")
        return False, f"exit {completed.returncode}"
    if step.mode == "empty_stdout" and (completed.stdout or "").strip():
        sys.stdout.write(completed.stdout)
        return False, "the command printed output where none is allowed"
    if step.mode == "quiet_unittest":
        print(_summarise_unittest(completed.stderr or ""))
    return True, ""


def execute(
    steps: Sequence[Step],
    *,
    environ: Mapping[str, str],
    root: Path = ROOT,
    clock: Callable[[], float] = time.monotonic,
    runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
    keep_going: bool = False,
    states: Mapping[str, object] | None = None,
    report_disk: bool = False,
) -> Verdict:
    started = clock()
    #: Where cargo will write, resolved once. `--report-disk` prints this
    #: directory's size after every step that could have grown it, so a run
    #: that dies of a full disk leaves a log that says how it got there.
    #: The 2026-08-20 CI failure left no such log, which is why this exists.
    target = footprint.target_directory(environ, root)
    #: Probed once per run. Injectable so a test can describe an environment
    #: this machine does not have — a tree with no private denylist, say —
    #: without the test's result depending on the machine it runs on.
    states = dict(evaluate_all(environ)) if states is None else dict(states)
    outcomes: list[StepOutcome] = []
    degradations: list[str] = []
    stop = False

    for step in steps:
        step_started = clock()
        if stop:
            outcomes.append(
                StepOutcome(step.key, step.label, "UNAVAILABLE", "an earlier step failed", 0.0)
            )
            continue
        missing = [name for name in step.requires if not states[name].available]
        if missing:
            reason = "; ".join(states[name].reason for name in missing)
            print(f"UNAVAILABLE {step.label} :: {reason}", flush=True)
            outcomes.append(
                StepOutcome(step.key, step.label, "UNAVAILABLE", reason, clock() - step_started)
            )
            continue

        argv_source = step.argv
        degraded = False
        if step.degrades_without and not states[step.degrades_without].available:
            argv_source = step.degraded_argv or step.argv
            degraded = True

        try:
            argv = expand_argv(argv_source, environ)
        except EnvironmentExpansionError as error:
            reason = str(error)
            print(f"UNAVAILABLE {step.label} :: {reason}", flush=True)
            outcomes.append(
                StepOutcome(step.key, step.label, "UNAVAILABLE", reason, clock() - step_started)
            )
            continue

        marker = " [DEGRADED]" if degraded else ""
        print(f"RUN {step.label}{marker} :: {' '.join(argv)}", flush=True)
        passed, detail = run_step(step, argv, root=root, environ=environ, runner=runner)
        seconds = clock() - step_started
        if degraded:
            degradations.append(f"{step.label}: {step.degraded_note}")
        outcomes.append(
            StepOutcome(
                step.key,
                step.label,
                "PASS" if passed else "FAIL",
                detail,
                seconds,
                degraded=degraded,
            )
        )
        if report_disk and step.owner in CARGO_OWNERS:
            print(f"     {footprint.describe(target)}", flush=True)
        if not passed:
            print(f"FAIL {step.label}: {detail}", file=sys.stderr, flush=True)
            if not keep_going:
                stop = True

    return Verdict(tuple(outcomes), tuple(degradations), clock() - started)


def print_verdict(verdict: Verdict, *, allow_unavailable: bool) -> None:
    print("")
    print("verification summary")
    print("--------------------")
    for outcome in verdict.outcomes:
        marker = " [DEGRADED]" if outcome.degraded else ""
        detail = f": {outcome.detail}" if outcome.detail else ""
        print(f"  {outcome.status:<11} {outcome.label}{marker}{detail} [{outcome.seconds:.3f}s]")
    print(f"  {'TOTAL':<11} [{verdict.seconds:.3f}s]")
    print("")
    if verdict.failed:
        for outcome in verdict.failed:
            print(f"FAILED :: {outcome.label}: {outcome.detail}")
        print(f"\nFAILED — {len(verdict.failed)} step(s) failed")
        return
    if verdict.complete:
        print("COMPLETE — every selected step ran and passed")
        return
    for outcome in verdict.unavailable:
        print(f"UNAVAILABLE :: {outcome.label}: {outcome.detail}")
    for note in verdict.degradations:
        print(f"DEGRADED :: {note}")
    tally = len(verdict.unavailable)
    print(
        f"\nINCOMPLETE — nothing failed, but {tally} step(s) could not run and "
        f"{len(verdict.degradations)} ran in a reduced form. This run does NOT "
        "prove what a complete one would."
    )
    if allow_unavailable:
        print(
            "--allow-unavailable was given, so this exits 0: the caller has "
            "declared it cannot supply what is missing."
        )
