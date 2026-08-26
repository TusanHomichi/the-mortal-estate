"""The step table: every command this repository can be proven with, once.

Two rules govern everything in this file.

**Only steps whose targets exist.** The predecessor's runner listed a Python
module that existed nowhere in its tree, and one scope failed on every run
because of it. That class of defect — the single source of truth for
verification naming something that is not there — is made impossible here by
`targets.py`, which is itself a step in this table.

**Every step is owned by exactly one scope.** The owner scopes below are a
partition of `STEPS`, asserted by `tests/test_verification_table.py`. The
composed lanes (`fast`, `portable`, `full`, `capture`) are built out of owner
scopes and can therefore never drift apart from the table or from each other.
"""

from __future__ import annotations

import sys
from pathlib import Path

from .capabilities import SYNTHETIC_TERMS
from .model import ResolutionError, Step

ROOT = Path(__file__).resolve().parents[2]

sys.path.insert(0, str(ROOT / "tools"))

import run_checks  # noqa: E402  (tools/ is on the path above)

# ---------------------------------------------------------------------------
# Python test ownership
# ---------------------------------------------------------------------------

#: Every `tests/test_*.py` module, classified by what it proves. The
#: classification FAILS CLOSED: `resolve.validate_python_ownership` refuses to
#: resolve any scope while a tracked module is unclassified or a classified
#: module has disappeared. Adding a test file without adding it here is a
#: resolution error, not a silently unrun test.
PYTHON_TEST_OWNERS: dict[str, tuple[str, ...]] = {
    "boundary": (
        "tests.test_boundary_common",
        "tests.test_check_boundary_terms",
        "tests.test_check_clean_room",
        "tests.test_check_hostnames",
        "tests.test_check_markdown_links",
        "tests.test_check_review_refs",
        "tests.test_run_checks",
    ),
    "workbench": (
        "tests.test_workbench_ambiguity",
        "tests.test_workbench_apply",
        "tests.test_workbench_fixture",
        "tests.test_workbench_imageops",
        "tests.test_workbench_loop",
        "tests.test_workbench_operations",
        "tests.test_workbench_parity",
        "tests.test_workbench_pointing",
        "tests.test_workbench_session",
        "tests.test_workbench_staleness",
    ),
    "capture": (
        "tests.test_capture_addressing",
        "tests.test_capture_correspondence",
        "tests.test_capture_sidecar",
    ),
    "harness": (
        "tests.test_live_proof_land",
        "tests.test_live_proof_pulse",
        "tests.test_pulse_capture",
        "tests.test_run_clean_clone_proof",
        "tests.test_run_gated_postgres",
    ),
    "verification": (
        "tests.test_agent_context",
        "tests.test_audio_provenance",
        "tests.test_ci_workflow",
        "tests.test_verification_execute",
        "tests.test_verification_footprint",
        "tests.test_verification_resolve",
        "tests.test_verification_table",
        "tests.test_verification_targets",
        "tests.test_working_root",
    ),
}


def python_test_step(owner: str) -> Step:
    modules = PYTHON_TEST_OWNERS[owner]
    return Step(
        key=f"python.{owner}",
        owner="python",
        label=f"python: {owner} tests",
        argv=("python3", "-m", "unittest", "-q", *modules),
        mode="quiet_unittest",
    )


# ---------------------------------------------------------------------------
# Boundary checks — enumerated from tools/run_checks.py, never re-listed here
# ---------------------------------------------------------------------------


def boundary_steps() -> tuple[Step, ...]:
    """One step per registered public-boundary check.

    `tools/run_checks.py` owns the registry. This reads it rather than
    restating it, so a fifth (or sixth) check joins the runner the moment it
    joins the registry and cannot be half-added.
    """
    steps = []
    for check in run_checks.CHECKS:
        # The link check is a boundary check by construction — same scan set,
        # same exit codes — but a documentation-only change must run it, so the
        # docs lane owns it and its key says so.
        owner = "docs" if check.name == "markdown-links" else "boundary"
        key = f"{owner}." + check.name.replace("-", "_")
        degrades = check.accepts_terms
        steps.append(
            Step(
                key=key,
                owner=owner,
                label=f"{owner}: {check.name}",
                argv=("python3", check.script),
                degrades_without="private-terms" if degrades else None,
                degraded_argv=(
                    ("python3", check.script, "--terms", SYNTHETIC_TERMS)
                    if degrades
                    else None
                ),
                degraded_note=(
                    "the real denylist is absent; the matching, scanning, and "
                    "fail-closed machinery ran against the tracked synthetic "
                    "fixture and asserts nothing about the real terms"
                    if degrades
                    else ""
                ),
            )
        )
    return tuple(steps)


# ---------------------------------------------------------------------------
# The table
# ---------------------------------------------------------------------------

_STATIC: tuple[Step, ...] = (
    Step(
        key="docs.routing",
        owner="docs",
        label="docs: subject routing agrees with the contract",
        argv=("python3", "tools/agent_context.py", "--validate"),
    ),
    Step(
        key="docs.whitespace",
        owner="docs",
        label="docs: git diff whitespace",
        argv=("git", "diff", "--check"),
    ),
    Step(
        key="rust.fmt",
        owner="rust",
        label="rust: formatting",
        argv=("cargo", "fmt", "--all", "--", "--check"),
    ),
    Step(
        key="rust.build",
        owner="rust",
        label="rust: workspace build",
        argv=("cargo", "build", "--workspace", "--locked"),
    ),
    Step(
        key="rust.clippy",
        owner="rust",
        label="rust: clippy",
        argv=("cargo", "clippy", "--workspace", "--locked", "--all-targets", "--", "-D", "warnings"),
    ),
    Step(
        key="rust.test",
        owner="rust",
        label="rust: workspace tests (bounded scheduler)",
        argv=("python3", "tools/run_rust_tests.py"),
    ),
    Step(
        key="python.compileall",
        owner="python",
        label="python: bytecode compilation",
        argv=("python3", "-m", "compileall", "-q", "tools", "tests"),
    ),
    Step(
        key="client.import",
        owner="client",
        label="client: engine import",
        argv=("$TME_GODOT", "--headless", "--path", "client", "--import"),
        requires=("godot",),
    ),
    Step(
        key="client.suite",
        owner="client",
        label="client: headless suite",
        argv=("$TME_GODOT", "--headless", "--path", "client", "-s", "res://tests/run_all.gd"),
        requires=("godot",),
    ),
    Step(
        key="workbench.demo",
        owner="workbench",
        label="workbench: scripted selection loop",
        argv=("python3", "tools/workbench_demo.py"),
    ),
    Step(
        key="gated.postgres",
        owner="gated",
        label="gated: PostgreSQL suite, one fresh migrated database per test",
        argv=("python3", "tools/run_gated_postgres.py", "--admin-url-file", "$TME_PG_ADMIN_URL_FILE"),
        requires=("postgres",),
        timeout=5400.0,
    ),
    Step(
        key="cleanclone.build_and_test",
        owner="cleanclone",
        label="clean clone: builds and tests with no private root",
        argv=("python3", "tools/run_clean_clone_proof.py"),
        timeout=5400.0,
    ),
    Step(
        key="meta.step_targets",
        owner="meta",
        label="meta: every step target exists",
        argv=("python3", "tools/run_verification.py", "--check-step-targets"),
    ),
    Step(
        key="capture.fixture_land",
        owner="capture",
        label="capture: fixture-land frame, identity raster, sidecar",
        argv=(
            "python3",
            "tools/run_fixture_land_capture.py",
            "--admin-url-file",
            "$TME_PG_ADMIN_URL_FILE",
            "--output",
            "$TME_CAPTURE_OUTPUT",
        ),
        requires=("godot", "postgres", "display", "capture-output"),
        timeout=1800.0,
    ),
    Step(
        key="capture.pulse",
        owner="capture",
        label="capture: the authoritative beat photographed advancing inside one round",
        argv=(
            "python3",
            "tools/run_pulse_capture.py",
            "--admin-url-file",
            "$TME_PG_ADMIN_URL_FILE",
            "--output",
            "$TME_CAPTURE_OUTPUT",
        ),
        requires=("godot", "postgres", "display", "capture-output"),
        timeout=1800.0,
    ),
    Step(
        key="capture.live_proof",
        owner="capture",
        label="capture: real client against a real server from an empty database",
        argv=(
            "python3",
            "tools/run_client_live_proof.py",
            "--admin-url-file",
            "$TME_PG_ADMIN_URL_FILE",
        ),
        requires=("godot", "postgres"),
        timeout=1800.0,
    ),
)


def build_steps() -> dict[str, Step]:
    steps: dict[str, Step] = {}
    for step in (
        *_STATIC,
        *boundary_steps(),
        *(python_test_step(owner) for owner in sorted(PYTHON_TEST_OWNERS)),
    ):
        if step.key in steps:
            raise ResolutionError(f"duplicate step key: {step.key}")
        steps[step.key] = step
    return dict(sorted(steps.items()))


STEPS: dict[str, Step] = build_steps()

#: The owner scopes. Every step belongs to exactly one; the partition test
#: proves it. `capture` is deliberately outside every standing composition —
#: it is owner-invoked, it needs a display and a database, and the charter's
#: fast loop exists precisely so nobody pays for it by accident.
OWNER_SCOPES: tuple[str, ...] = (
    "docs",
    "boundary",
    "python",
    "rust",
    "workbench",
    "client",
    "gated",
    "cleanclone",
    "meta",
    "capture",
)

#: Scopes that are owner-invoked and never part of a standing composition.
OUT_OF_BASELINE: frozenset[str] = frozenset({"capture"})

#: The scopes that make cargo write to a target directory, and therefore the
#: only ones whose disk cost is worth measuring. `--report-disk` prints the
#: footprint after each of their steps; measuring after a markdown link check
#: would be a `du` over gigabytes to learn nothing.
CARGO_OWNERS: frozenset[str] = frozenset({"rust", "cleanclone"})

#: Composed lanes. `@name` includes another scope. The order here is the order
#: steps run in: cheap and diagnostic first, expensive last.
COMPOSED: dict[str, tuple[str, ...]] = {
    "portable": ("@meta", "@docs", "@boundary", "@python", "@rust", "@workbench"),
    "full": ("@portable", "@client", "@gated", "@cleanclone"),
}

#: Standing rule, encoded: the fast lane is defined by what it excludes.
#: Nothing in this set may ever appear in a `fast` resolution, whatever the
#: changed paths were, unless the matching family was actually touched.
FAST_FORBIDDEN_WITHOUT_CAUSE: frozenset[str] = frozenset(
    {"rust", "client", "gated", "cleanclone", "capture"}
)

ALL_SCOPES: tuple[str, ...] = tuple(sorted({*OWNER_SCOPES, *COMPOSED, "fast"}))
