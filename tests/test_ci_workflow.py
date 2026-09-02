"""CI names lanes; this asserts the lanes it names still add up to `full`.

The workflow lists no steps of its own — that is the property that makes "CI
passes" and "it passes locally" one claim rather than two. But once CI is
allowed to run the complete lane as **more than one job**, a second way to
drift opens: the jobs can stop covering `full` between them, and nothing about
either job would look wrong. A dropped `--scope cleanclone` is a merge gate
that silently stopped running.

So the composition is asserted here, against the resolved step table rather
than against the scope names: the union of the steps CI's executing
invocations resolve to must equal `full`'s steps exactly, and no step may be
paid for twice.

The reader below is deliberately small. It reads four things out of one file
in this repository — the top-level `env:` block, the job names, each job's
`run:` lines, and the scopes they name — and adding a YAML parser to the
Python corpus to do that would widen the stack for four keys. The conventions
it depends on are asserted, not assumed: every runner invocation lives on a
single `run:` line, so none can hide inside a block this does not read.
"""

from __future__ import annotations

import re
import tomllib
import unittest

from verification_test_support import REPO_ROOT

from verification import resolve
from verification.footprint import LEAN_BUILD_ENV

WORKFLOW = REPO_ROOT / ".github" / "workflows" / "verify.yml"
CARGO_MANIFEST = REPO_ROOT / "Cargo.toml"
RUST_TOOLCHAIN = REPO_ROOT / "rust-toolchain.toml"

#: The tracked script every building job runs before it spends anything.
BUDGET_SCRIPT_NAME = ".github/disk-budget.sh"

#: The tracked script that provisions the pinned toolchain — the first thing in
#: a job that costs real disk.
INSTALL_SCRIPT_NAME = ".github/install-rust.sh"

#: Current Node 24 action majors, selected from their official releases on
#: 2026-08-27. A future runtime migration changes these deliberately.
CHECKOUT_ACTION = "actions/checkout@v7"
SETUP_PYTHON_ACTION = "actions/setup-python@v7"

RUNNER = "tools/run_verification.py"
_RUN_LINE = re.compile(r"^\s*run:\s*(python3 " + re.escape(RUNNER) + r".*?)\s*$")
_USES_LINE = re.compile(r"^\s*-\s+uses:\s*(\S+)\s*$")
_JOB_HEADER = re.compile(r"^  ([A-Za-z_][A-Za-z0-9_-]*):\s*$")
_ENV_ENTRY = re.compile(r"^  ([A-Za-z_][A-Za-z0-9_]*):\s*(.*?)\s*$")
_SCOPE = re.compile(r"--scope\s+(\S+)")


def _lines() -> list[str]:
    return WORKFLOW.read_text(encoding="utf-8").splitlines()


def workflow_environment() -> dict[str, str]:
    """The top-level `env:` block — the one every job inherits."""
    found: dict[str, str] = {}
    collecting = False
    for line in _lines():
        if line == "env:":
            collecting = True
            continue
        if not collecting:
            continue
        if line and not line.startswith(" "):
            break
        if not line.strip() or line.strip().startswith("#"):
            continue
        match = _ENV_ENTRY.match(line)
        if match is None:
            continue
        found[match.group(1)] = match.group(2).strip().strip('"').strip("'")
    return found


def job_blocks() -> dict[str, list[str]]:
    """Each job's lines, keyed by job name."""
    blocks: dict[str, list[str]] = {}
    current: str | None = None
    inside = False
    for line in _lines():
        if line == "jobs:":
            inside = True
            continue
        if not inside:
            continue
        if line and not line.startswith(" "):
            break
        header = _JOB_HEADER.match(line)
        if header is not None:
            current = header.group(1)
            blocks[current] = []
            continue
        if current is not None:
            blocks[current].append(line)
    return blocks


def runner_commands(block: list[str]) -> list[str]:
    """Every runner invocation in a job, as its command line."""
    commands = []
    for line in block:
        if line.strip().startswith("#"):
            continue
        match = _RUN_LINE.match(line)
        if match is not None:
            commands.append(match.group(1))
    return commands


def action_uses(block: list[str]) -> tuple[str, ...]:
    """Every action reference from an actual `uses:` step, in workflow order."""
    found = []
    for line in block:
        match = _USES_LINE.match(line)
        if match is not None:
            found.append(match.group(1))
    return tuple(found)


def scopes_of(command: str) -> tuple[str, ...]:
    return tuple(_SCOPE.findall(command))


class TheWorkflowIsReadable(unittest.TestCase):
    """The conventions the rest of this suite depends on, asserted first."""

    def test_the_workflow_exists(self) -> None:
        self.assertTrue(WORKFLOW.is_file(), WORKFLOW)

    def test_every_runner_invocation_is_a_single_run_line(self) -> None:
        for name, block in job_blocks().items():
            for line in block:
                stripped = line.strip()
                if RUNNER not in stripped or stripped.startswith("#"):
                    continue
                self.assertIsNotNone(
                    _RUN_LINE.match(line),
                    f"{name}: {stripped!r} names the runner somewhere this suite "
                    "cannot read; keep every invocation on one `run:` line",
                )

    def test_at_least_one_job_runs_the_runner(self) -> None:
        self.assertTrue(any(runner_commands(block) for block in job_blocks().values()))

    def test_every_action_reference_is_a_single_uses_line(self) -> None:
        for name, block in job_blocks().items():
            for line in block:
                if line.strip().startswith("- uses:"):
                    self.assertIsNotNone(_USES_LINE.match(line), f"{name}: {line!r}")

    def test_action_reader_ignores_a_comment_that_names_the_expected_pin(self) -> None:
        block = [
            "      # uses: actions/checkout@v7",
            "      - uses: actions/checkout@v8",
        ]
        self.assertEqual(action_uses(block), ("actions/checkout@v8",))


class TheJobsCoverTheCompleteLane(unittest.TestCase):
    def executing(self) -> dict[str, tuple[str, ...]]:
        """Job name -> the scopes it actually runs (`--list` is a plan, not a run)."""
        return {
            name: tuple(
                scope
                for command in runner_commands(block)
                if "--list" not in command
                for scope in scopes_of(command)
            )
            for name, block in job_blocks().items()
            if any("--list" not in command for command in runner_commands(block))
        }

    def test_the_executed_lanes_are_exactly_the_complete_lane(self) -> None:
        covered: set[str] = set()
        for scopes in self.executing().values():
            covered |= {step.key for step in resolve.steps_for(list(scopes))}
        self.assertEqual(covered, {step.key for step in resolve.steps_for(["full"])})

    def test_no_step_is_paid_for_twice(self) -> None:
        seen: set[str] = set()
        for name, scopes in self.executing().items():
            keys = {step.key for step in resolve.steps_for(list(scopes))}
            overlap = sorted(seen & keys)
            self.assertFalse(overlap, f"{name} re-runs {overlap}")
            seen |= keys

    def test_no_executing_invocation_resolves_by_changed_path(self) -> None:
        """CI proves everything. A changed-path lane there would prove a subset."""
        for name, block in job_blocks().items():
            for command in runner_commands(block):
                self.assertNotIn("--changed-path", command, name)
                self.assertNotIn("--scope fast", command, name)

    def test_every_job_prints_the_plan_it_is_about_to_run(self) -> None:
        for name, scopes in self.executing().items():
            planned = [
                scopes_of(command)
                for command in runner_commands(job_blocks()[name])
                if "--list" in command
            ]
            self.assertEqual(
                planned,
                [scopes],
                f"{name} must print exactly the lane it runs, before running it",
            )

    def test_every_executing_job_declares_it_cannot_supply_every_capability(self) -> None:
        for name, block in job_blocks().items():
            for command in runner_commands(block):
                if "--list" in command:
                    continue
                self.assertIn("--allow-unavailable", command, name)


class TheActionRuntimePinsAreCurrent(unittest.TestCase):
    """Every job uses the reviewed Node 24 action majors exactly once."""

    def test_every_job_uses_exactly_the_reviewed_action_steps(self) -> None:
        for name, block in job_blocks().items():
            self.assertEqual(
                action_uses(block),
                (CHECKOUT_ACTION, SETUP_PYTHON_ACTION),
                name,
            )


class TheDiskBudgetIsVisible(unittest.TestCase):
    """The 2026-08-20 failure, made impossible to repeat silently.

    Both attempts at run 32438837232 died of `No space left on device` with no
    step log at all. A job that builds must therefore say what it has and what
    it is spending, in its own log, before and as it spends it.
    """

    def building_jobs(self) -> dict[str, list[str]]:
        from verification.table import CARGO_OWNERS

        found = {}
        for name, block in job_blocks().items():
            for command in runner_commands(block):
                if "--list" in command:
                    continue
                owners = {
                    step.owner for step in resolve.steps_for(list(scopes_of(command)))
                }
                if owners & CARGO_OWNERS:
                    found[name] = block
        return found

    def test_a_building_job_states_its_budget_before_it_spends_anything(self) -> None:
        for name, block in self.building_jobs().items():
            text = "\n".join(block)
            self.assertIn(BUDGET_SCRIPT_NAME, text, f"{name} never states its disk budget")
            spending = next(
                index
                for index, line in enumerate(block)
                if RUNNER in line or INSTALL_SCRIPT_NAME in line
            )
            stating = next(
                index for index, line in enumerate(block) if BUDGET_SCRIPT_NAME in line
            )
            self.assertLess(stating, spending, f"{name} spends disk before counting it")

    def test_the_budget_script_measures_both_sides_of_what_it_frees(self) -> None:
        script = (REPO_ROOT / BUDGET_SCRIPT_NAME).read_text(encoding="utf-8")
        self.assertGreaterEqual(script.count("df -h"), 2)
        for reclaimed in ("dotnet", "android", "ghc", "CodeQL"):
            self.assertIn(reclaimed, script, f"the budget script does not free {reclaimed}")

    def test_a_building_job_reports_the_footprint_of_every_build_step(self) -> None:
        for name, block in self.building_jobs().items():
            for command in runner_commands(block):
                if "--list" in command:
                    continue
                self.assertIn("--report-disk", command, name)


class TheLeanProfileIsDeclaredOnce(unittest.TestCase):
    """One source of truth for the disposable-build profile, and CI reads it.

    `verification.footprint.LEAN_BUILD_ENV` is where the profile is decided.
    The workflow restates it because GitHub Actions has no way to import a
    Python dict — so this asserts the restatement is faithful, which is the
    difference between a copy and a drift.
    """

    def test_the_workflow_declares_the_same_profile(self) -> None:
        declared = workflow_environment()
        for name, value in LEAN_BUILD_ENV.items():
            self.assertEqual(declared.get(name), value, name)

    def test_the_target_directory_is_named_so_the_log_can_measure_it(self) -> None:
        self.assertIn("CARGO_TARGET_DIR", workflow_environment())


class TheRustToolchainPinsAgree(unittest.TestCase):
    """Local selection, Cargo's floor, and CI must name one baseline."""

    def test_the_exact_local_and_ci_pins_match_the_workspace_floor(self) -> None:
        toolchain = tomllib.loads(RUST_TOOLCHAIN.read_text(encoding="utf-8"))[
            "toolchain"
        ]
        workspace = tomllib.loads(CARGO_MANIFEST.read_text(encoding="utf-8"))[
            "workspace"
        ]["package"]
        exact = toolchain["channel"]

        self.assertEqual(workflow_environment().get("RUST_TOOLCHAIN_VERSION"), exact)
        self.assertEqual(workspace["rust-version"], exact.removesuffix(".0"))

    def test_the_local_pin_carries_the_proof_components(self) -> None:
        toolchain = tomllib.loads(RUST_TOOLCHAIN.read_text(encoding="utf-8"))[
            "toolchain"
        ]

        self.assertEqual(toolchain["profile"], "minimal")
        self.assertEqual(set(toolchain["components"]), {"clippy", "rustfmt"})


if __name__ == "__main__":
    unittest.main()
