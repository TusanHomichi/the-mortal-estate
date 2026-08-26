"""Replay the staged log against the accepted master, and say what came back.

Replay is the shared half of preview and Apply. Both resolve the same effective
operation set from the same log, re-verify the same digests, and hand the same
document to the same validator — so "what the preview showed" and "what Apply
did" are one code path with two destinations, not two implementations that agree
today.

**The order of the steps is the safety.**

1. Re-verify every bound digest first: the session's own base binding, and the
   selection packet each staged operation derives from. Any staleness aborts
   before anything is computed, because a stale packet is a precise instruction
   about a world that has moved.
2. Replay the operations, in log order, against a COPY of the accepted master.
   The copy is made by the compiler, which reads the tracked master as bytes and
   writes the candidate into the session directory this module hands it.
3. Judge the candidate with the compiler's own semantics, and — for a preview —
   ask the compiler for the candidate's logical view.

**Determinism.** The same log against the same base produces byte-identical
output. The mutation is a pure function of the two, the serializer is the
compiler's single one, and the effective set is a derivation of the log rather
than a thing anyone edits. `tests/test_workbench_apply.py` proves it by replaying
twice and comparing digests, and by replaying an empty log and getting the
accepted master back byte for byte.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from . import bridge
from . import operations as operation_log
from .projection import Source, StaleSelection, WorkbenchError, verify
from .session import PREVIEW_DIR, Session

OPERATIONS_NAME = "operations.json"

#: What a replay was doing when it stopped. Carried on every outcome so a
#: rejection record can say which step refused rather than only that one did.
STAGE_STALENESS = "staleness"
STAGE_REPLAY = "replay"
STAGE_VALIDATE = "validate"
STAGE_ACCEPTED = "accepted"

MASTER_ROLE = "master"


@dataclass(frozen=True)
class Outcome:
    """One replay, and everything a receipt or a rejection needs to say."""

    accepted: bool
    stage: str
    detail: dict
    operations: list[dict]
    base: dict
    candidate: dict | None = None
    projection: dict | None = None

    def as_record(self) -> dict:
        return {
            "accepted": self.accepted,
            "stage": self.stage,
            "detail": self.detail,
            "operations": operation_log.summary(self.operations),
            "base": self.base,
            "candidate": self.candidate,
            "candidate_projection": self.projection,
        }


def land_of(session: Session) -> str:
    """Which land this session edits, as the session bound it when it opened.

    Read off the manifest for the same reason the base binding is: a session is
    an edit OF something, and a session that names no land is refused rather
    than defaulted onto whichever land the compiler happens to carry first.
    """
    land = session.manifest.get("land_id")
    if not isinstance(land, str) or not land.strip():
        raise WorkbenchError(
            "this session's manifest names no land; it cannot be the base of an edit"
        )
    return land


def base_binding(session: Session) -> dict:
    """The accepted master, as the session bound it when it opened.

    Read off the manifest rather than the disk, because the point of a binding
    is to be compared against the disk. A session whose manifest names no master
    is refused rather than defaulted — there is no safe guess about which bytes
    an edit is an edit OF.
    """
    for record in session.manifest.get("base_digests", []):
        if record.get("role") == MASTER_ROLE:
            return {"path": record["path"], "sha256": record["sha256"]}
    raise WorkbenchError(
        "this session's manifest binds no accepted master; it cannot be the base "
        "of an edit"
    )


def bound_sources(session: Session, operations) -> list[Source]:
    """Every digest a replay of this set depends on, with no duplicates.

    The session's own base binding, plus the full bound set of every selection
    packet the staged operations derive from. Each packet is verified as a
    whole: a capture-view packet binds eight files and every one of them is
    checked, because a packet is one address and half an address is not one.
    """
    sources: list[Source] = [
        Source.from_record(record) for record in session.manifest.get("base_digests", [])
    ]
    seen = {(source.path, source.sha256) for source in sources}
    for record in operations:
        packet = session.read_packet(record["selection_id"])
        for entry in packet["source"]["digests"]:
            source = Source.from_record(entry)
            key = (source.path, source.sha256)
            if key not in seen:
                seen.add(key)
                sources.append(source)
    return sources


def run(
    session: Session,
    *,
    directory: Path,
    project: bool = False,
) -> Outcome:
    """Verify, replay, judge. Writes only inside the session directory."""
    root = session.root
    operations = session.staged()
    base = base_binding(session)
    truth = operation_log.by_class(operations, operation_log.CLASS_TRUTH)

    try:
        verify(root, bound_sources(session, operations))
    except StaleSelection as stale:
        return Outcome(
            accepted=False,
            stage=STAGE_STALENESS,
            detail={"error": str(stale), "moved": stale.moved},
            operations=operations,
            base=base,
        )

    payload = operation_log.truth_operation_set(truth)
    operations_path = directory / OPERATIONS_NAME
    operations_path.parent.mkdir(parents=True, exist_ok=True)
    operations_path.write_bytes((json.dumps(payload, indent=2) + "\n").encode("utf-8"))

    answer = bridge.replay(
        root,
        land=land_of(session),
        operations=operations_path,
        output_directory=directory,
        expect_base_sha256=base["sha256"],
        validate=True,
        project=project,
    )
    document = answer.document
    if document.get("kind") == "workbench_replay_refused":
        return Outcome(
            accepted=False,
            stage=STAGE_REPLAY,
            detail=document,
            operations=operations,
            base=base,
        )
    if not answer.yes:
        # The replay produced a document and the compiler refused it. The
        # validator's own words are what the owner needs, verbatim.
        return Outcome(
            accepted=False,
            stage=STAGE_VALIDATE,
            detail=document.get("report", document),
            operations=operations,
            base=base,
            candidate=_relative(root, document["candidate"]),
        )
    return Outcome(
        accepted=True,
        stage=STAGE_ACCEPTED,
        detail=document.get("report", {}),
        operations=operations,
        base=base,
        candidate=_relative(root, document["candidate"]),
        projection=_relative(root, document["projection"]) if document.get("projection") else None,
    )


def _relative(root: Path, record: dict) -> dict:
    """Name an artifact by its repository-relative path where it has one."""
    path = Path(record["path"])
    try:
        path = path.relative_to(Path(root).resolve())
    except ValueError:
        pass
    return {"path": str(path), "sha256": record["sha256"]}


def preview(session: Session) -> Outcome:
    """Re-derive the candidate's logical view without touching `generated/`.

    The preview lands in the session's own `preview/` directory and is
    overwritten every time. It is derived, disposable state: the log is what
    persists, and the candidate is a function of it.
    """
    directory = session.artifact(PREVIEW_DIR)
    return run(session, directory=directory, project=True)
