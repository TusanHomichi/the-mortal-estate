"""Turn a request — scopes, or changed paths — into an ordered list of steps.

The charter's four loops become lanes here. The load-bearing rule is the
charter's own: **a full workspace build must not be the price of inspecting a
visual adjustment.** So the fast lane is defined by what it EXCLUDES. A path
family that was not touched cannot pull in its scope, and `assert_fast_lane`
enforces that mechanically rather than by reading the table carefully.

The escape hatch is deliberate and loud: anything this module does not
recognise escalates to the complete portable baseline and says why. A guess is
never cheaper than a build.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

from .model import ResolutionError, Step
from .table import (
    COMPOSED,
    FAST_FORBIDDEN_WITHOUT_CAUSE,
    OUT_OF_BASELINE,
    OWNER_SCOPES,
    PYTHON_TEST_OWNERS,
    ROOT,
    STEPS,
)

#: Documentation lives in several roots; markdown anywhere is documentation.
DOC_ROOT_FILES = frozenset({"AGENTS.md", "CLAUDE.md", "README.md", "LICENSE"})

#: Paths whose change could alter what any other path means. Each escalates to
#: the portable baseline rather than being classified.
CONSERVATIVE_PREFIXES = (
    ".github/",
    ".cargo/",
    "tools/verification/",
    "deploy/production/",
)
CONSERVATIVE_EXACT = frozenset(
    {
        ".gitignore",
        "Cargo.lock",
        "Cargo.toml",
        "tools/run_verification.py",
        "tools/run_rust_tests.py",
        "tools/run_checks.py",
        "tools/boundary_common.py",
        "tools/agent_context.py",
    }
)

#: Which scopes each recognised family needs. `meta` is added to every
#: resolution: it costs milliseconds and it is what keeps this table honest.
FAMILY_SCOPES: dict[str, tuple[str, ...]] = {
    "docs": ("docs", "boundary"),
    "python": ("python", "boundary"),
    "boundary-data": ("boundary", "python"),
    "rust": ("rust",),
    "authoring": ("rust", "workbench", "python", "boundary"),
    "content": ("rust", "boundary"),
    "client": ("client",),
    "workbench": ("workbench", "python"),
}

FAMILY_REASONS: dict[str, str] = {
    "docs": "documentation paths select the docs lane and the boundary checks",
    "python": "tool and test paths select the Python suite and the boundary checks",
    "boundary-data": "boundary list data selects the boundary checks and their tests",
    "rust": "Rust paths select the workspace lane",
    "authoring": (
        "the authoring compiler owns the operation vocabulary the Workbench "
        "bridges into, and an authored land is what a candidate is an edit "
        "OF, so its Python proof runs beside the workspace lane"
    ),
    "content": "authored content is validated by the Rust workspace",
    "client": "client paths select the headless client suite",
    "workbench": "Workbench paths select the Workbench proof and the Python suite",
}


@dataclass(frozen=True)
class Selection:
    scopes: tuple[str, ...]
    reasons: tuple[str, ...]
    escalated: bool = False


# ---------------------------------------------------------------------------
# Ownership: fails closed on an unclassified test module
# ---------------------------------------------------------------------------


def validate_python_ownership(*, root: Path = ROOT) -> None:
    """Refuse to resolve anything while the Python test inventory has a hole.

    A test module nobody classified is a module no lane runs. Discovering that
    at review time is too late, so it is a resolution error here: every scope,
    including `docs`, refuses until the inventory is whole again.
    """
    classified: list[str] = [
        module for modules in PYTHON_TEST_OWNERS.values() for module in modules
    ]
    duplicates = sorted({name for name in classified if classified.count(name) > 1})
    if duplicates:
        raise ResolutionError(f"Python test modules classified twice: {duplicates}")
    discovered = {f"tests.{path.stem}" for path in (root / "tests").glob("test_*.py")}
    unclassified = sorted(discovered - set(classified))
    vanished = sorted(set(classified) - discovered)
    if unclassified or vanished:
        raise ResolutionError(
            "the Python test ownership inventory disagrees with tests/: "
            f"unclassified={unclassified}, classified-but-missing={vanished}"
        )


# ---------------------------------------------------------------------------
# Scope expansion
# ---------------------------------------------------------------------------


def expand(scopes: Sequence[str]) -> tuple[str, ...]:
    """Expand composed scopes into owner scopes, order preserved, deduplicated."""
    ordered: list[str] = []

    def walk(name: str, seen: frozenset[str]) -> None:
        if name in seen:
            raise ResolutionError(f"scope composition cycles at {name!r}")
        if name in COMPOSED:
            for member in COMPOSED[name]:
                walk(member[1:] if member.startswith("@") else member, seen | {name})
            return
        if name not in OWNER_SCOPES:
            raise ResolutionError(f"unknown scope: {name}")
        if name not in ordered:
            ordered.append(name)

    for scope in scopes:
        walk(scope, frozenset())
    return tuple(ordered)


def steps_for(scopes: Sequence[str], *, root: Path = ROOT) -> tuple[Step, ...]:
    validate_python_ownership(root=root)
    owners = expand(scopes)
    order = {owner: index for index, owner in enumerate(owners)}
    selected = [step for step in STEPS.values() if step.owner in order]
    selected.sort(key=lambda step: (order[step.owner], step.key))
    return tuple(selected)


# ---------------------------------------------------------------------------
# Changed-path resolution — the fast lane
# ---------------------------------------------------------------------------


def _is_safe(path: str) -> bool:
    if not path or any(character in path for character in "\n\r\0"):
        return False
    candidate = Path(path)
    return not candidate.is_absolute() and all(
        part not in {"", ".", ".."} for part in path.split("/")
    )


def classify(path: str) -> str | None:
    """Return the family a repository-relative path belongs to, or None."""
    if path in DOC_ROOT_FILES or path.endswith(".md"):
        return "docs"
    if path.startswith("tests/fixtures/workbench/") or path.startswith("tools/workbench/"):
        return "workbench"
    if path == "tools/workbench_demo.py":
        return "workbench"
    if path.startswith("tools/") and (path.endswith("-allowlist.txt") or "terms" in path):
        return "boundary-data"
    if (path.startswith("tools/") or path.startswith("tests/")) and path.endswith(".py"):
        return "python"
    if (
        path.startswith("crates/tme-authoring/")
        or path.startswith("content/authoring-fixture/")
        or path.startswith("content/lands/")
    ):
        return "authoring"
    if path.startswith("crates/") or path.startswith(".sqlx/"):
        return "rust"
    if path.startswith("content/"):
        return "content"
    if path.startswith("client/"):
        return "client"
    if path.startswith("tests/fixtures/"):
        return "python"
    return None


def select_changed_paths(paths: Sequence[str], *, root: Path = ROOT) -> Selection:
    """Resolve the fast lane, escalating loudly rather than guessing."""
    if not paths:
        return Selection(
            ("portable",),
            ("no changed paths given; the complete portable baseline is the safe answer",),
            escalated=True,
        )
    families: list[str] = []
    for path in paths:
        if not _is_safe(path):
            return Selection(
                ("portable",), (f"malformed path {path!r} escalates to portable",), True
            )
        if path in CONSERVATIVE_EXACT or path.startswith(CONSERVATIVE_PREFIXES):
            return Selection(
                ("portable",),
                (f"{path} can change what every other path means; escalates to portable",),
                True,
            )
        if not (root / path).exists():
            return Selection(
                ("portable",),
                (f"{path} was deleted or renamed; escalates to portable",),
                True,
            )
        family = classify(path)
        if family is None:
            return Selection(
                ("portable",),
                (f"unrecognised path {path} escalates to portable",),
                True,
            )
        if family not in families:
            families.append(family)

    scopes: list[str] = ["meta"]
    reasons: list[str] = []
    for family in families:
        reasons.append(FAMILY_REASONS[family])
        for scope in FAMILY_SCOPES[family]:
            if scope not in scopes:
                scopes.append(scope)
    selection = Selection(tuple(scopes), tuple(reasons))
    assert_fast_lane(selection, families)
    return selection


def assert_fast_lane(selection: Selection, families: Sequence[str]) -> None:
    """The exclusion rule, enforced rather than trusted.

    An expensive owner scope may appear in a fast resolution only because a
    path family that needs it was actually touched. This is the mechanical form
    of the charter's rule that a workspace build is not the price of inspecting
    a change; without it the fast lane drifts toward the complete one one
    convenient addition at a time.
    """
    if selection.escalated:
        return
    for scope in OUT_OF_BASELINE:
        if scope in selection.scopes:
            raise ResolutionError(
                f"{scope!r} is owner-invoked and must never be selected automatically"
            )
    caused: set[str] = set()
    for family in families:
        caused.update(FAMILY_SCOPES[family])
    for scope in selection.scopes:
        if scope in FAST_FORBIDDEN_WITHOUT_CAUSE and scope not in caused:
            raise ResolutionError(
                f"the fast lane selected {scope!r} with no changed path that needs it"
            )
