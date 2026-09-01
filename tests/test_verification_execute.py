"""The availability contract, asserted.

One rule, tested from every side: **UNAVAILABLE is never PASS.** A run that
could not prove something says so, exits 3, and only exits 0 when the caller
explicitly declares — with `--allow-unavailable` — that it cannot supply what
is missing.
"""

from __future__ import annotations

import io
import subprocess
import unittest
from contextlib import redirect_stdout
from dataclasses import dataclass
from unittest.mock import patch

from verification_test_support import REPO_ROOT  # noqa: F401  (path setup)

from verification import capabilities, execute
from verification.model import (
    CapabilityState,
    EXIT_FAILED,
    EXIT_INCOMPLETE,
    EXIT_OK,
    EnvironmentExpansionError,
    Step,
    StepOutcome,
    Verdict,
    expand_argv,
)


@dataclass
class FakeCompleted:
    returncode: int = 0
    stdout: str = ""
    stderr: str = ""


def fake_runner(codes: dict[str, int]):
    calls: list[list[str]] = []

    def runner(argv, **_kwargs):
        calls.append(list(argv))
        return FakeCompleted(returncode=codes.get(argv[0], 0))

    runner.calls = calls  # type: ignore[attr-defined]
    return runner


def clock_from(values):
    iterator = iter(values)
    return lambda: next(iterator)


ALWAYS = Step("ok.one", "docs", "always runs", ("always",))
NEEDS_GODOT = Step("client.x", "client", "needs the client", ("godot",), requires=("godot",))


class NodeCapability(unittest.TestCase):
    def test_node_is_a_reported_capability(self) -> None:
        self.assertIn("node", capabilities.BY_NAME)

    def test_node_22_and_npm_make_the_capability_available(self) -> None:
        def versions(argv, **_kwargs):
            reported = (
                "v22.4.1\n"
                if argv[-1] == "--version" and "node" in argv[0]
                else "10.8.1\n"
            )
            return FakeCompleted(stdout=reported)

        with (
            patch.object(capabilities.shutil, "which", side_effect=["/fake/node", "/fake/npm"]),
            patch.object(capabilities.subprocess, "run", side_effect=versions),
        ):
            state = capabilities.BY_NAME["node"].evaluate({"PATH": "/fake"})
        self.assertTrue(state.available, state.reason)
        self.assertIn("npm", state.reason)


class UnavailableIsNeverPass(unittest.TestCase):
    def environ(self, **extra) -> dict[str, str]:
        return {"PATH": "/usr/bin", **extra}

    def test_a_step_whose_capability_is_absent_does_not_run(self) -> None:
        runner = fake_runner({})
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [NEEDS_GODOT], environ=self.environ(), runner=runner
            )
        self.assertEqual(runner.calls, [])
        self.assertEqual(verdict.outcomes[0].status, "UNAVAILABLE")

    def test_an_unavailable_step_makes_the_run_incomplete(self) -> None:
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute([NEEDS_GODOT], environ=self.environ(), runner=fake_runner({}))
        self.assertFalse(verdict.complete)
        self.assertEqual(verdict.exit_code(allow_unavailable=False), EXIT_INCOMPLETE)

    def test_allow_unavailable_is_the_only_way_to_exit_zero_incomplete(self) -> None:
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute([NEEDS_GODOT], environ=self.environ(), runner=fake_runner({}))
        self.assertEqual(verdict.exit_code(allow_unavailable=True), EXIT_OK)

    def test_a_failure_outranks_allow_unavailable(self) -> None:
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [ALWAYS, NEEDS_GODOT],
                environ=self.environ(),
                runner=fake_runner({"always": 1}),
            )
        self.assertEqual(verdict.exit_code(allow_unavailable=True), EXIT_FAILED)

    def test_a_complete_run_exits_zero(self) -> None:
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute([ALWAYS], environ=self.environ(), runner=fake_runner({}))
        self.assertTrue(verdict.complete)
        self.assertEqual(verdict.exit_code(allow_unavailable=False), EXIT_OK)

    def test_the_reason_a_capability_is_absent_is_reported(self) -> None:
        stream = io.StringIO()
        with redirect_stdout(stream):
            execute.execute([NEEDS_GODOT], environ=self.environ(), runner=fake_runner({}))
        self.assertIn("TME_GODOT is not set", stream.getvalue())


#: An environment with everything except the private denylist — a clean clone.
NO_PRIVATE_TERMS = {
    "godot": CapabilityState("godot", True, "pinned"),
    "postgres": CapabilityState("postgres", True, "present"),
    "private-terms": CapabilityState("private-terms", False, ".boundary/ is absent"),
    "display": CapabilityState("display", True, "present"),
    "capture-output": CapabilityState("capture-output", True, "present"),
}


class Degradation(unittest.TestCase):
    def test_a_degraded_step_runs_its_reduced_command(self) -> None:
        step = Step(
            "boundary.x",
            "boundary",
            "degrades",
            ("real",),
            degrades_without="private-terms",
            degraded_argv=("real", "--terms", "synthetic"),
            degraded_note="asserts nothing about the real list",
        )
        runner = fake_runner({})
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [step], environ={"PATH": "/usr/bin"}, runner=runner, states=NO_PRIVATE_TERMS
            )
        self.assertEqual(runner.calls, [["real", "--terms", "synthetic"]])
        self.assertEqual(verdict.outcomes[0].status, "PASS")
        self.assertTrue(verdict.outcomes[0].degraded)

    def test_a_degraded_run_is_not_complete(self) -> None:
        step = Step(
            "boundary.x",
            "boundary",
            "degrades",
            ("real",),
            degrades_without="private-terms",
            degraded_argv=("real", "--terms", "synthetic"),
            degraded_note="asserts nothing about the real list",
        )
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [step],
                environ={"PATH": "/usr/bin"},
                runner=fake_runner({}),
                states=NO_PRIVATE_TERMS,
            )
        self.assertFalse(verdict.complete)
        self.assertEqual(verdict.exit_code(allow_unavailable=False), EXIT_INCOMPLETE)
        self.assertIn("asserts nothing about the real list", verdict.degradations[0])

    def test_a_present_denylist_means_no_degradation(self) -> None:
        step = Step(
            "boundary.x",
            "boundary",
            "degrades",
            ("real",),
            degrades_without="private-terms",
            degraded_argv=("real", "--terms", "synthetic"),
            degraded_note="asserts nothing about the real list",
        )
        present = dict(NO_PRIVATE_TERMS)
        present["private-terms"] = CapabilityState("private-terms", True, "present")
        runner = fake_runner({})
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [step], environ={"PATH": "/usr/bin"}, runner=runner, states=present
            )
        self.assertEqual(runner.calls, [["real"]])
        self.assertTrue(verdict.complete)


class FailureStopsTheRun(unittest.TestCase):
    def test_a_failure_stops_later_steps_by_default(self) -> None:
        second = Step("ok.two", "docs", "second", ("second",))
        runner = fake_runner({"always": 2})
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [ALWAYS, second], environ={"PATH": "/usr/bin"}, runner=runner
            )
        self.assertEqual(runner.calls, [["always"]])
        self.assertEqual(verdict.outcomes[1].status, "UNAVAILABLE")
        self.assertEqual(verdict.outcomes[1].detail, "an earlier step failed")

    def test_keep_going_runs_everything(self) -> None:
        second = Step("ok.two", "docs", "second", ("second",))
        runner = fake_runner({"always": 2})
        with redirect_stdout(io.StringIO()):
            execute.execute(
                [ALWAYS, second],
                environ={"PATH": "/usr/bin"},
                runner=runner,
                keep_going=True,
            )
        self.assertEqual(runner.calls, [["always"], ["second"]])


class Timing(unittest.TestCase):
    def test_every_step_is_timed_and_so_is_the_run(self) -> None:
        with redirect_stdout(io.StringIO()):
            verdict = execute.execute(
                [ALWAYS],
                environ={"PATH": "/usr/bin"},
                runner=fake_runner({}),
                clock=clock_from([100.0, 100.5, 102.0, 104.0]),
            )
        self.assertAlmostEqual(verdict.outcomes[0].seconds, 1.5)
        self.assertAlmostEqual(verdict.seconds, 4.0)


class ExpansionAndTimeouts(unittest.TestCase):
    def test_an_unset_variable_refuses_to_expand(self) -> None:
        with self.assertRaises(EnvironmentExpansionError):
            expand_argv(("$NOT_SET", "x"), {})

    def test_an_empty_variable_refuses_to_expand(self) -> None:
        with self.assertRaises(EnvironmentExpansionError):
            expand_argv(("$EMPTY",), {"EMPTY": ""})

    def test_expansion_substitutes_inside_a_token(self) -> None:
        self.assertEqual(expand_argv(("$D/out.png",), {"D": "/tmp"}), ("/tmp/out.png",))

    def test_a_timeout_is_a_failure_not_a_hang(self) -> None:
        def timing_out(argv, **kwargs):
            raise subprocess.TimeoutExpired(argv, kwargs.get("timeout", 1))

        passed, detail = execute.run_step(
            ALWAYS, ("always",), environ={"PATH": "/usr/bin"}, runner=timing_out
        )
        self.assertFalse(passed)
        self.assertIn("ceiling", detail)

    def test_a_missing_executable_is_a_failure_not_a_crash(self) -> None:
        def exploding(argv, **_kwargs):
            raise OSError("no such file")

        passed, detail = execute.run_step(
            ALWAYS, ("always",), environ={"PATH": "/usr/bin"}, runner=exploding
        )
        self.assertFalse(passed)
        self.assertIn("no such file", detail)


class TheVerdictReads(unittest.TestCase):
    def test_an_incomplete_verdict_says_it_proves_less(self) -> None:
        verdict = Verdict(
            (StepOutcome("a", "a step", "UNAVAILABLE", "no database", 0.0),), (), 1.0
        )
        stream = io.StringIO()
        with redirect_stdout(stream):
            execute.print_verdict(verdict, allow_unavailable=False)
        self.assertIn("INCOMPLETE", stream.getvalue())
        self.assertIn("does NOT", stream.getvalue())

    def test_a_complete_verdict_says_so_plainly(self) -> None:
        verdict = Verdict((StepOutcome("a", "a step", "PASS", "", 0.1),), (), 1.0)
        stream = io.StringIO()
        with redirect_stdout(stream):
            execute.print_verdict(verdict, allow_unavailable=False)
        self.assertIn("COMPLETE", stream.getvalue())
        self.assertNotIn("INCOMPLETE", stream.getvalue())


class TheRunSaysWhatItIsSpending(unittest.TestCase):
    """`--report-disk`, which exists because a run once died without saying so.

    Reported after the steps that build and only those: a `du` over gigabytes
    to learn that a markdown link check changed nothing is a cost with no
    reader.
    """

    def _output(self, step: Step, *, report_disk: bool) -> str:
        stream = io.StringIO()
        with redirect_stdout(stream):
            execute.execute(
                (step,),
                environ={"PATH": "/usr/bin", "CARGO_TARGET_DIR": str(REPO_ROOT / "target")},
                clock=clock_from([0.0, 0.0, 0.1, 0.2]),
                runner=fake_runner({}),
                report_disk=report_disk,
                states={
                    name: CapabilityState(name, True, "supplied")
                    for name in ("godot", "postgres", "private-terms", "display", "capture-output")
                },
            )
        return stream.getvalue()

    def test_a_build_step_is_followed_by_its_footprint(self) -> None:
        building = Step("rust.x", "rust", "builds something", ("cargo",))
        self.assertIn("disk ::", self._output(building, report_disk=True))

    def test_a_step_that_builds_nothing_is_not_measured(self) -> None:
        self.assertNotIn("disk ::", self._output(ALWAYS, report_disk=True))

    def test_nothing_is_measured_unless_the_caller_asked(self) -> None:
        building = Step("rust.x", "rust", "builds something", ("cargo",))
        self.assertNotIn("disk ::", self._output(building, report_disk=False))


if __name__ == "__main__":
    unittest.main()
