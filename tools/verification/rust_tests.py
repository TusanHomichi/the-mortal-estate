"""Compile the workspace's tests once, then run the targets through a bounded pool.

Cargo's own `cargo test` runs one test binary at a time while each binary runs
its cases in parallel. The binaries are independent processes with no shared
state, so the outer serialisation is pure wall-clock cost. This compiles once
with `--no-run --message-format=json`, reads the targets cargo reports, and runs
them concurrently — **bounded**, because the machine also has to serve whatever
else the run is doing.

**Each target runs through `cargo test`, not by executing the binary directly.**
That is the whole design note, and it was paid for: running the binaries
directly bypasses cargo's `[env]` table, so `.cargo/config.toml`'s
`TME_BANNED_TERMS_FILE` never reached the tests and
`tme_sim::loading::tests::diagnostics_preserve_nested_component_and_json_pointer_ownership`
failed against the wrong denylist. A test runner that silently changes the
environment the tests were written for is worse than a slow one: it produces a
red that has nothing to do with the code, or — on a differently configured
machine — a green that means nothing. Cargo owns the test environment; asking it
to launch each target is how that stays true without this file mirroring cargo's
configuration rules.

Once the compile pass is done, per-target `cargo test` invocations do not
serialise on the build directory: measured on this workspace, two concurrent
invocations finished in less wall time than either one took alone.

Two things it refuses to do loosely:

* **Every exit code is checked.** A target that dies on a signal, or cannot be
  launched at all, is a failure with its output printed — never a target whose
  absence from the summary goes unnoticed.
* **Doctests still run.** They are not test targets and cargo does not report
  them as artifacts, so they run as their own `--doc` pass rather than being
  quietly dropped from the count.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Mapping

ROOT = Path(__file__).resolve().parents[2]

#: Two at a time. Enough to halve the wall clock on this workspace, small
#: enough that a `--scope full` run does not starve the machine it is on.
DEFAULT_JOBS = 2

#: Target kinds cargo builds a test harness for, and the selector each needs.
_SELECTOR = {"lib": ("--lib",), "bin": ("--bin",), "test": ("--test",)}


class RustTestError(RuntimeError):
    """Compilation, or cargo's report of it, could not be used."""


@dataclass(frozen=True)
class TestTarget:
    """One test harness cargo built, and how to ask cargo to run exactly it."""

    package: str
    kind: str
    name: str

    @property
    def label(self) -> str:
        return f"{self.package}::{self.name}"

    def selector(self) -> tuple[str, ...]:
        flag = _SELECTOR[self.kind]
        return flag if self.kind == "lib" else (*flag, self.name)

    def command(self, cargo: str) -> tuple[str, ...]:
        return (
            cargo,
            "test",
            "--locked",
            "-p",
            self.package,
            *self.selector(),
            "--",
            "--quiet",
        )


@dataclass(frozen=True)
class TestResult:
    target: TestTarget
    returncode: int
    stdout: str
    stderr: str
    seconds: float


def parse_artifacts(raw: str) -> tuple[TestTarget, ...]:
    """Read cargo's JSON artifact stream into the set of test targets.

    The package name comes from `manifest_path`'s directory rather than from
    `package_id`, whose shape cargo has changed more than once.
    """
    found: dict[tuple[str, str, str], TestTarget] = {}
    for number, line in enumerate(raw.splitlines(), start=1):
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError as error:
            raise RustTestError(f"cargo artifact line {number} is not JSON: {error}") from error
        if message.get("reason") != "compiler-artifact":
            continue
        profile = message.get("profile")
        target = message.get("target")
        if not isinstance(profile, dict) or not profile.get("test"):
            continue
        if message.get("executable") is None:
            continue
        manifest = message.get("manifest_path")
        if not isinstance(target, dict) or not isinstance(manifest, str):
            raise RustTestError(f"cargo artifact line {number} lacks target identity")
        kinds = [kind for kind in target.get("kind", []) if kind in _SELECTOR]
        if not kinds and set(target.get("kind", [])) <= {"rlib", "cdylib"} and target.get("kind"):
            kinds = ["lib"]
        name = target.get("name")
        if not kinds or not isinstance(name, str):
            raise RustTestError(
                f"cargo artifact line {number} has a test harness of no runnable kind: "
                f"{target.get('kind')!r}"
            )
        package = Path(manifest).parent.name
        candidate = TestTarget(package, kinds[0], name)
        found[(package, kinds[0], name)] = candidate
    if not found:
        raise RustTestError("cargo reported no test targets")
    return tuple(sorted(found.values(), key=lambda item: (item.package, item.kind, item.name)))


def compile_tests(
    cargo: str,
    *,
    environ: Mapping[str, str],
    runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
) -> tuple[TestTarget, ...]:
    completed = runner(
        [cargo, "test", "--workspace", "--locked", "--no-run", "--message-format=json"],
        cwd=str(ROOT),
        env=dict(environ),
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        print(completed.stdout or "", end="")
        print(completed.stderr or "", end="")
        raise RustTestError(f"test compilation failed with exit {completed.returncode}")
    return parse_artifacts(completed.stdout)


def run_one(
    target: TestTarget,
    *,
    cargo: str,
    environ: Mapping[str, str],
    runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
    clock: Callable[[], float] = time.monotonic,
) -> TestResult:
    started = clock()
    try:
        completed = runner(
            list(target.command(cargo)),
            cwd=str(ROOT),
            env=dict(environ),
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as error:
        return TestResult(target, 127, "", str(error), clock() - started)
    return TestResult(
        target,
        completed.returncode,
        completed.stdout or "",
        completed.stderr or "",
        clock() - started,
    )


def _summary_line(result: TestResult) -> str:
    tail = [
        line.strip()
        for line in result.stdout.splitlines()
        if line.strip().startswith("test result:")
    ]
    status = "PASS" if result.returncode == 0 else "FAIL"
    detail = tail[-1] if tail else "completed"
    return f"{status} {result.target.label} [{result.seconds:.3f}s] {detail}"


def execute(
    *,
    jobs: int = DEFAULT_JOBS,
    environ: Mapping[str, str] | None = None,
    runner: Callable[..., subprocess.CompletedProcess] = subprocess.run,
) -> int:
    environ = dict(os.environ if environ is None else environ)
    cargo = environ.get("CARGO", "cargo")
    started = time.monotonic()
    try:
        targets = compile_tests(cargo, environ=environ, runner=runner)
    except (OSError, RustTestError) as error:
        print(f"FAIL rust test compilation: {error}")
        return 1
    print(f"{len(targets)} test targets, {jobs} at a time")
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        results = [
            future.result()
            for future in [
                pool.submit(run_one, target, cargo=cargo, environ=environ, runner=runner)
                for target in targets
            ]
        ]
    failed = False
    for result in results:
        print(_summary_line(result))
        if result.returncode != 0:
            failed = True
            print(result.stdout, end="")
            print(result.stderr, end="")
    doctests = runner(
        [cargo, "test", "--workspace", "--locked", "--doc", "--quiet"],
        cwd=str(ROOT),
        env=dict(environ),
        capture_output=True,
        text=True,
        check=False,
    )
    if doctests.returncode != 0:
        failed = True
        print(f"FAIL doctests: exit {doctests.returncode}")
        print(doctests.stdout or "", end="")
        print(doctests.stderr or "", end="")
    else:
        print("PASS doctests")
    print(
        f"{'FAIL' if failed else 'PASS'} rust workspace tests "
        f"[{time.monotonic() - started:.3f}s]"
    )
    return 1 if failed else 0
