"""The typed vocabulary the verification spine is built from.

Three ideas live here and nothing else:

**A capability** is something the *environment* either provides or does not —
the pinned client binary, a PostgreSQL superuser URL, the private denylist, a
usable display. A capability is probed once per run and the answer is recorded
with a reason, so an absent one is reported as an honest UNAVAILABLE rather
than inferred from a step that quietly did nothing.

**A step** is one command with one exit code, owned by exactly one scope. A
step may require capabilities; a step may also *degrade* — run a reduced form
that still proves its mechanism when a capability is missing — and a degraded
run is reported as degraded, never as a clean pass.

**A verdict** distinguishes the two ways a run can end without a failure:
COMPLETE (everything selected actually ran) and INCOMPLETE (nothing failed,
but something could not be proven here). The exit codes follow
`tools/boundary_common.py`'s vocabulary, which this repository already speaks.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, Mapping, Sequence

#: Exit codes. Deliberately the same three the boundary checks use.
EXIT_OK = 0
EXIT_FAILED = 1
EXIT_USAGE = 2
EXIT_INCOMPLETE = 3


class ResolutionError(RuntimeError):
    """The requested scopes, paths, or step table cannot be resolved."""


class EnvironmentExpansionError(RuntimeError):
    """A step's argv names an environment variable that is not set."""


@dataclass(frozen=True)
class Capability:
    """One environmental prerequisite, and how to ask whether it is here.

    `probe` returns `(available, reason)`. The reason is printed either way:
    when available it says what was found, when absent it says exactly what is
    missing and how to supply it. A capability never raises — an exploding
    probe is an unavailable capability with the explosion as its reason.
    """

    name: str
    summary: str
    probe: Callable[[Mapping[str, str]], tuple[bool, str]]

    def evaluate(self, environ: Mapping[str, str]) -> "CapabilityState":
        try:
            available, reason = self.probe(environ)
        except Exception as error:  # noqa: BLE001 - a broken probe is an absent capability
            available, reason = False, f"probe failed: {error}"
        return CapabilityState(self.name, available, reason)


@dataclass(frozen=True)
class CapabilityState:
    name: str
    available: bool
    reason: str


@dataclass(frozen=True)
class Step:
    """One verification command.

    `owner` is the single scope that owns this step. The partition test in
    `tests/test_verification_table.py` asserts every step has exactly one, so
    a step cannot be quietly claimed by two lanes or by none.

    `requires` names capabilities the step cannot run without: absent any one
    of them the step is UNAVAILABLE and is not executed.

    `degraded_argv` is the reduced form for when `degrades_without` is absent.
    A step that degrades still runs and still must pass — what changes is the
    claim it supports, which is recorded in the verdict as a degradation.
    """

    key: str
    owner: str
    label: str
    argv: tuple[str, ...]
    mode: str = "command"
    requires: tuple[str, ...] = ()
    degrades_without: str | None = None
    degraded_argv: tuple[str, ...] | None = None
    degraded_note: str = ""
    #: Minutes are the wrong unit for most steps; this is a hard ceiling in
    #: seconds so a hung child cannot hold a lane open forever.
    timeout: float = 3600.0

    def __post_init__(self) -> None:
        if (self.degrades_without is None) != (self.degraded_argv is None):
            raise ResolutionError(
                f"{self.key}: degrades_without and degraded_argv come as a pair"
            )


@dataclass(frozen=True)
class StepOutcome:
    """What one step did. `status` is PASS, FAIL, or UNAVAILABLE."""

    key: str
    label: str
    status: str
    detail: str
    seconds: float
    degraded: bool = False


@dataclass(frozen=True)
class Verdict:
    """The whole run, reduced to something a human or a gate can read."""

    outcomes: tuple[StepOutcome, ...]
    degradations: tuple[str, ...]
    seconds: float

    @property
    def failed(self) -> tuple[StepOutcome, ...]:
        return tuple(item for item in self.outcomes if item.status == "FAIL")

    @property
    def unavailable(self) -> tuple[StepOutcome, ...]:
        return tuple(item for item in self.outcomes if item.status == "UNAVAILABLE")

    @property
    def complete(self) -> bool:
        return not self.unavailable and not self.degradations

    def exit_code(self, *, allow_unavailable: bool) -> int:
        if self.failed:
            return EXIT_FAILED
        if self.complete or allow_unavailable:
            return EXIT_OK
        return EXIT_INCOMPLETE


def expand_argv(argv: Sequence[str], environ: Mapping[str, str]) -> tuple[str, ...]:
    """Substitute `$NAME` tokens from the environment.

    An unset or empty variable raises rather than expanding to nothing: a step
    that would run with a hole in its command line is a step that would prove
    something other than what its label claims.
    """
    import re

    reference = re.compile(r"\$([A-Za-z_][A-Za-z0-9_]*)")

    def replace(match: "re.Match[str]") -> str:
        name = match.group(1)
        value = environ.get(name)
        if not value:
            raise EnvironmentExpansionError(f"{name} is not set")
        return value

    return tuple(reference.sub(replace, token) for token in argv)
