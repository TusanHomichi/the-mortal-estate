"""Prove that every target the step table names actually exists.

The predecessor's runner listed a Python module that existed nowhere in its
repository. One scope failed on every run, and the failure was invisible until
somebody ran that scope. The single source of truth for verification pointing
at a target that is not there should not be a defect that gets fixed once — it
should be a defect that cannot survive a run.

So this is a step in the table, and the table is its input. Its own mutant
receipt is in `docs/boundary-checks.md`: a step naming a module that does not
exist is planted, and this reports it.
"""

from __future__ import annotations

from pathlib import Path
from typing import Iterable, Mapping

from .capabilities import BY_NAME
from .model import Step
from .table import OWNER_SCOPES, ROOT

#: Tokens that are options or literals, never paths.
_NOT_A_PATH = frozenset(
    {
        "--",
        "-D",
        "-q",
        "-s",
        "-p",
        "--all",
        "--check",
        "--headless",
        "--locked",
        "--workspace",
        "--all-targets",
        "--import",
        "--quiet",
        "--version",
        "--check-step-targets",
        "warnings",
        "unittest",
        "compileall",
        "python3",
        "cargo",
        "npm",
        "git",
        "diff",
        "build",
        "test",
        "fmt",
        "clippy",
        "ci",
        "run",
        "typecheck",
    }
)

#: Extensions that make a token unambiguously a repository path.
_PATH_SUFFIXES = (".py", ".txt", ".json", ".gd", ".toml", ".md")


def _module_path(token: str) -> str | None:
    if token.startswith("tests.") and token.count(".") == 1:
        return token.replace(".", "/") + ".py"
    return None


def _looks_like_path(token: str) -> bool:
    if token in _NOT_A_PATH or token.startswith("$") or token.startswith("-"):
        return False
    if token.endswith(_PATH_SUFFIXES):
        return True
    return "/" in token


def step_targets(step: Step) -> list[str]:
    """Every repository path this step's argv names, as relative paths."""
    targets: list[str] = []
    previous = ""
    for token in step.argv:
        module = _module_path(token)
        if module is not None:
            targets.append(module)
        elif previous in {"--path", "--prefix"}:
            targets.append(token)
        elif _looks_like_path(token):
            targets.append(token)
        elif previous in {"compileall", "-q"} and token in {"tools", "tests"}:
            targets.append(token)
        previous = token
    return targets


def missing_targets(
    steps: Iterable[Step], *, root: Path = ROOT
) -> list[str]:
    """Every problem in the step table, as one-line diagnostics.

    Three classes, because all three are the same defect wearing different
    clothes: a step that names a target that is not there, a step that requires
    a capability nobody defined, and a step owned by a scope that does not
    exist. Each makes the table lie about what a run proves.
    """
    problems: list[str] = []
    for step in steps:
        if step.owner not in OWNER_SCOPES:
            problems.append(f"{step.key}: owner {step.owner!r} is not an owner scope")
        for capability in (*step.requires, *( (step.degrades_without,) if step.degrades_without else () )):
            if capability not in BY_NAME:
                problems.append(f"{step.key}: unknown capability {capability!r}")
        for argv in (step.argv, step.degraded_argv or ()):
            for target in step_targets(Step(step.key, step.owner, step.label, tuple(argv))):
                if not (root / target).exists():
                    problems.append(f"{step.key}: names a target that does not exist: {target}")
    return sorted(set(problems))


def report(steps: Mapping[str, Step] | Iterable[Step], *, root: Path = ROOT) -> int:
    """Print the check's result and return an exit code."""
    values = steps.values() if hasattr(steps, "values") else steps
    problems = missing_targets(values, root=root)
    if not problems:
        print("step-targets: OK")
        return 0
    print(f"step-targets: {len(problems)} problem(s)")
    for problem in problems:
        print(f"  {problem}")
    return 1
