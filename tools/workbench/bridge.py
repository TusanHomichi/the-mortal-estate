"""The one bridge from the Workbench into the compiler's semantics.

Everything else in this package reads and writes files. This module runs a
program — the authoring compiler — because the compiler is where the meaning of
an authored member lives, and there is exactly one of it. A Python copy of the
surface's semantics, or of the derivation that keeps its derived layers true,
would be a second opinion about what an authored document is; the authoring
standard names that class of duplication as the false-green failure it exists to
kill. So the session, the log, the ordering, and the digests are this package's,
and the document is the compiler's.

**This is the package's only module that starts programs.** Authored-document
verdicts come from the Rust compiler. Explicit edit operations pay that cost;
pointing, resolving, reading packets, and listing sessions start nothing.

**What it never does.** It never promotes. `validate-candidate` reads no
promotion receipt and consults no reviewed digest; `replay` reads the accepted
master as bytes and writes a candidate wherever this package tells it to, which
is always inside the disposable session directory. No subcommand reachable from
here writes tracked content — the one that does (`--check` / no arguments) is
not called from this package at all.
"""

from __future__ import annotations

import json
import os
import subprocess
from dataclasses import dataclass
from pathlib import Path

from .projection import WorkbenchError

#: The exact command. Named here so a test can read it argument by argument
#: rather than trusting a sentence about it.
COMPILER_COMMAND = ("cargo", "run", "--quiet", "--locked", "-p", "tme-authoring", "--")

#: Long, because a cold `target/` means a real build before the first answer.
#: The ordinary case is a rebuild-check and costs a fraction of a second.
COMPILER_TIMEOUT_SECONDS = 900.0

#: The compiler's own exit contract (`crates/tme-authoring/src/cli.rs`).
EXIT_YES = 0
EXIT_NO = 1
EXIT_UNREADABLE = 2


class CompilerUnavailable(WorkbenchError):
    """The compiler could not be run, or could not be understood.

    Distinct from a NO answer on purpose: "the candidate is rejected" and "the
    validator never ran" are different facts, and a tool that conflated them
    would report a clean tree as proven.
    """


@dataclass(frozen=True)
class Answer:
    """One compiler answer: the document it printed, and whether it said yes."""

    yes: bool
    document: dict

    def require_yes(self, what: str) -> dict:
        if not self.yes:
            raise CompilerUnavailable(f"{what}: {json.dumps(self.document, indent=2)}")
        return self.document


def _run(root: Path, arguments: list[str]) -> Answer:
    try:
        finished = subprocess.run(
            [*COMPILER_COMMAND, *arguments],
            cwd=Path(root),
            capture_output=True,
            text=True,
            timeout=COMPILER_TIMEOUT_SECONDS,
            # A build that asks a question would hang a tool nobody is watching.
            stdin=subprocess.DEVNULL,
            env={**os.environ, "CARGO_TERM_COLOR": "never"},
        )
    except FileNotFoundError as error:
        raise CompilerUnavailable(
            "the authoring compiler could not be started: "
            f"{COMPILER_COMMAND[0]} is not on the path. "
            "The Workbench reaches the compiler's semantics through it and has no "
            "second implementation to fall back on."
        ) from error
    except subprocess.TimeoutExpired as error:
        raise CompilerUnavailable(
            f"the authoring compiler did not answer within "
            f"{COMPILER_TIMEOUT_SECONDS:.0f}s"
        ) from error

    if finished.returncode == EXIT_UNREADABLE:
        raise CompilerUnavailable(
            f"the authoring compiler refused the request: {finished.stderr.strip()}"
        )
    if finished.returncode not in (EXIT_YES, EXIT_NO):
        raise CompilerUnavailable(
            f"the authoring compiler exited {finished.returncode}: "
            f"{finished.stderr.strip() or finished.stdout.strip()}"
        )
    try:
        document = json.loads(finished.stdout)
    except json.JSONDecodeError as error:
        raise CompilerUnavailable(
            f"the authoring compiler printed something that is not JSON: {error}. "
            f"stderr: {finished.stderr.strip()}"
        ) from error
    return Answer(yes=finished.returncode == EXIT_YES, document=document)


def _target(land: str, member: str | None) -> list[str]:
    """Which land, and which of its members, a request addresses.

    The land is required at every entry point and never defaulted here: the
    compiler carries more than one, and a tool that guessed would eventually
    edit a document nobody asked about. The member is optional because the
    compiler derives it from the land's single candidate entry point, which is
    a derivation from the contract rather than a guess.
    """
    if not land:
        raise CompilerUnavailable("a compiler request must name the land it addresses")
    arguments = ["--land", land]
    if member is not None:
        arguments += ["--member", member]
    return arguments


def describe_operations(root: Path, land: str, member: str | None = None) -> dict:
    """The truth-operation vocabulary, from the crate that owns it."""
    return _run(root, ["describe-operations", *_target(land, member)]).require_yes(
        "the compiler could not describe its operation vocabulary"
    )


def validate_candidate(
    root: Path, document: Path, land: str, member: str | None = None
) -> Answer:
    """Run the accepted master's semantics against a candidate document.

    A NO here is an ordinary answer, not a failure: it is the compiler saying
    the candidate breaks a rule, in the same words the promoted path would use.
    """
    return _run(root, ["validate-candidate", *_target(land, member), str(document)])


def project_candidate(
    root: Path, document: Path, land: str, member: str | None = None
) -> Answer:
    """The candidate's logical view, for a preview of what Apply would produce."""
    return _run(root, ["project-candidate", *_target(land, member), str(document)])


def replay(
    root: Path,
    *,
    land: str,
    member: str | None = None,
    operations: Path,
    output_directory: Path,
    expect_base_sha256: str,
    validate: bool = True,
    project: bool = False,
) -> Answer:
    """Replay a truth-operation set against the accepted master.

    `expect_base_sha256` is required rather than defaulted, all the way down:
    replaying against bytes the caller did not expect is exactly how a stale
    session quietly edits a document nobody looked at.
    """
    arguments = [
        "replay",
        *_target(land, member),
        "--operations",
        str(operations),
        "--output-dir",
        str(output_directory),
        "--expect-base-sha256",
        expect_base_sha256,
    ]
    if validate:
        arguments.append("--validate")
    if project:
        arguments.append("--project")
    return _run(root, arguments)
