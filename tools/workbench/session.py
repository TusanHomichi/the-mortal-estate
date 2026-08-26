"""The session directory — a shared screen that is a shared directory.

A session is plain files under an ignored root. The owner's browser writes
them; an agent reads them with ordinary file tools; nothing tracked depends on
them. That is what makes agent parity mechanical rather than aspirational, and
it is why the session is not a database, a socket, or an in-memory state.

**Sessions are disposable working state.** They carry no project authority, no
promotion power, and no runtime meaning. A session left open for a week commits
to nothing; deleting one loses nothing but the working notes in it.

**The operation log is one file.** V0 wrote two record kinds — a selection was
taken, the owner said something — with an `operation` field that was always
null. V1 fills that field and adds three more kinds beside it: an operation was
retracted, an Apply happened, the owner accepted a candidate. One file, appended
to in one order, because the order IS the semantics: replay is the log, in the
order the log has.

**Nothing this file writes leaves the session directory.** Every artifact path
goes through [`Session.artifact`], which refuses a path that escapes. That is
the mechanical form of the rule that the Workbench cannot write tracked content:
not a convention about where callers point, but a refusal at the one place
paths are made.
"""

from __future__ import annotations

import json
import os
import random
import shutil
import string
import time
from dataclasses import dataclass
from pathlib import Path

from . import VERSION
from . import operations as operation_log
from .packet import MASKED_GESTURES, build, cells_for_gesture, geometry_of, mask_bytes, now
from .projection import Projection, WorkbenchError, digest_bytes

SESSION_ROOT = ".workbench/sessions"
MANIFEST_NAME = "manifest.json"
OPERATIONS_NAME = "operations.jsonl"
SELECTIONS_DIR = "selections"
MASKS_DIR = "masks"
#: Commit masks — the pixels an asset edit may replace. A directory of their
#: own because they are in a different address space from the selection masks
#: beside them: a selection mask covers CELLS of the land, a commit mask covers
#: PIXELS of a picture. Same file format, and one glob over one directory would
#: hand a consumer both while telling it nothing about which was which.
COMMIT_MASKS_DIR = "commit"
#: Where a preview's candidate lands: derived, disposable, overwritten freely.
PREVIEW_DIR = "preview"
#: Where an Apply's outputs and receipts land, one directory per attempt.
APPLY_DIR = "apply"

MANIFEST_KIND = "workbench_session_manifest"
SELECTION_RECORD = "selection_recorded"
COMMENT_RECORD = "owner_comment"
SCHEMA_VERSION = 1

#: Ruled at genesis plan Phase 8 in docs/working-root-policy.md, closing
#: Workbench spec §13 open decision 2. Longer than any plausible single piece of
#: work, shorter than the interval at which anyone would think to look.
RETENTION_DAYS = 14
#: However recent they are, this many sessions is the ceiling.
RETENTION_KEEP = 10

RETENTION_STATEMENT = (
    "Disposable working state. Sessions live under an ignored root, are never "
    "tracked, are never runtime input, and are never an authority. Delete any "
    "session directory at any time; nothing references it. Convention: keep the "
    "session you are working in, drop the rest — 'rm -rf .workbench/sessions' is "
    f"always a safe command in this repository. Automatic cleanup: sessions "
    f"older than {RETENTION_DAYS} days, and any session beyond the most recent "
    f"{RETENTION_KEEP}, are removed by tools/workbench_prune.py."
)


def repository_revision(root: Path) -> str | None:
    """The checked-out commit, read from `.git` directly.

    Read rather than shelled out for, because serving and selecting must invoke
    no external process at all. Advisory: it orients a reader and never decides
    staleness.
    """
    git = Path(root) / ".git"
    try:
        head = (git / "HEAD").read_text(encoding="utf-8").strip()
    except OSError:
        return None
    if not head.startswith("ref:"):
        return head or None
    reference = head.split(":", 1)[1].strip()
    try:
        return (git / reference).read_text(encoding="utf-8").strip() or None
    except OSError:
        pass
    try:
        packed = (git / "packed-refs").read_text(encoding="utf-8")
    except OSError:
        return None
    for line in packed.splitlines():
        if line.startswith("#") or not line.strip():
            continue
        parts = line.split()
        if len(parts) == 2 and parts[1] == reference:
            return parts[0]
    return None


def _write(path: Path, payload: bytes) -> None:
    """Write whole or not at all, so a reading agent never sees half a packet."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".partial")
    temporary.write_bytes(payload)
    os.replace(temporary, path)


def _json_bytes(value) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=False) + "\n").encode("utf-8")


def new_session_id() -> str:
    stamp = now().replace("-", "").replace(":", "")
    suffix = "".join(random.choice(string.ascii_lowercase + string.digits) for _ in range(6))
    return f"session-{stamp}-{suffix}"


@dataclass
class Session:
    """One open session directory."""

    root: Path
    directory: Path
    session_id: str
    manifest: dict

    @property
    def relative(self) -> str:
        return str(self.directory.relative_to(self.root))

    @property
    def candidate_member(self) -> str:
        """The member this session's land accepts truth operations against.

        Bound when the session opened, from the compiler's own projection. A
        session that names none is refused rather than defaulted: there is no
        safe guess about which member an edit is an edit OF.
        """
        member = self.manifest.get("candidate_member")
        if not isinstance(member, str) or not member.strip():
            raise WorkbenchError(
                "this session's manifest names no candidate member; it cannot "
                "stage a truth operation"
            )
        return member

    def _packet_path(self, selection_id: str) -> Path:
        return self.directory / SELECTIONS_DIR / f"{selection_id}.json"

    def _mask_path(self, selection_id: str) -> Path:
        return self.directory / MASKS_DIR / f"{selection_id}.pbm"

    def selection_ids(self) -> list[str]:
        directory = self.directory / SELECTIONS_DIR
        if not directory.is_dir():
            return []
        return sorted(path.stem for path in directory.glob("*.json"))

    def next_selection_id(self) -> str:
        existing = self.selection_ids()
        return f"sel-{len(existing) + 1:04d}"

    def read_packet(self, selection_id: str) -> dict:
        path = self._packet_path(selection_id)
        try:
            return json.loads(path.read_bytes())
        except OSError as error:
            raise WorkbenchError(f"no packet {selection_id!r} in this session") from error

    def write_selection(self, packet: dict, mask: bytes | None) -> dict:
        """Write the packet, its mask, and the log record that says it happened."""
        selection_id = packet["selection_id"]
        if mask is not None:
            mask_path = self._mask_path(selection_id)
            _write(mask_path, mask)
            reference = {
                "path": str(mask_path.relative_to(self.root)),
                "sha256": digest_bytes(mask),
                "format": "pbm_p1_over_cell_bounds",
            }
            packet["screen_region"]["mask"] = reference
        path = self._packet_path(selection_id)
        _write(path, _json_bytes(packet))
        self.append(
            {
                "schema_version": SCHEMA_VERSION,
                "kind": SELECTION_RECORD,
                "record_id": self.next_record_id(),
                "recorded_at": now(),
                "author": packet.get("author", "owner"),
                "selection_id": selection_id,
                "packet": f"{SELECTIONS_DIR}/{selection_id}.json",
                # The seam the staged-operation log extends in V1. V0a stages
                # nothing, so it is null here and a reader can rely on that.
                "operation": None,
            }
        )
        return packet

    def record_logical_selection(self, projection: Projection, body: dict) -> dict:
        """Take one logical selection and write it, packet, mask, and log record.

        The browser's route and the agent's command line both arrive here. Agent
        parity is a law, and two code paths that each assembled a packet would be
        two chances to assemble it differently — which is precisely the class of
        difference nobody notices until a packet resolves to the wrong place.
        """
        member = projection.member(str(body["member"]))
        gesture = str(body["gesture"])
        cells = cells_for_gesture(member, gesture, body)
        packet = build(
            projection=projection,
            member=member,
            gesture=gesture,
            cells=cells,
            screen_region=body.get("canvas_rect"),
            comment=str(body.get("comment", "")),
            selection_id=self.next_selection_id(),
            created_at=now(),
            repository_revision=self.manifest.get("repository_revision"),
            mask_reference=None,
            geometry=geometry_of(gesture, body),
            author=str(body.get("author", "owner")),
        )
        mask = mask_bytes(member, cells) if gesture in MASKED_GESTURES else None
        packet = self.write_selection(packet, mask)
        if packet["comment"]:
            self.write_comment(packet["selection_id"], packet["comment"])
        return packet

    def write_comment(self, selection_id: str | None, comment: str) -> dict:
        record = {
            "schema_version": SCHEMA_VERSION,
            "kind": COMMENT_RECORD,
            "record_id": self.next_record_id(),
            "recorded_at": now(),
            "author": "owner",
            "selection_id": selection_id,
            # Verbatim. Never parsed for facts, by anything, ever.
            "comment": comment,
            "operation": None,
        }
        self.append(record)
        return record

    def next_record_id(self) -> str:
        return f"op-{len(self.operations()) + 1:04d}"

    def append(self, record: dict) -> None:
        path = self.directory / OPERATIONS_NAME
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a", encoding="utf-8") as stream:
            stream.write(json.dumps(record, sort_keys=False) + "\n")

    # -- V1: staged operations, artifacts, receipts ----------------------

    def artifact(self, *parts) -> Path:
        """A path inside this session, or a refusal.

        Every V1 output — a candidate document, a candidate projection, an
        edited asset, a receipt, a rejection record — is addressed through
        here. The Workbench may not write tracked content, and this is the
        single place that is enforced rather than assumed. A caller that
        escapes with `..` gets a refusal naming what it tried.
        """
        target = (self.directory / Path(*parts)).resolve()
        root = self.directory.resolve()
        if target != root and root not in target.parents:
            raise WorkbenchError(
                f"{target} is outside the session directory {root}; the Workbench "
                "writes nothing anywhere else"
            )
        return target

    def write_artifact(self, relative, payload: bytes) -> dict:
        """Write one artifact atomically and describe it by path and digest."""
        path = self.artifact(relative)
        _write(path, payload)
        return {
            "path": str(path.relative_to(self.root)),
            "sha256": digest_bytes(payload),
        }

    def stage_operation(self, record: dict) -> dict:
        """Append one staged operation. Validated here, judged by the compiler."""
        operation_log.validate(record, editable_member=self.candidate_member)
        self.append(record)
        return record

    def retract_operation(self, retracts: str, reason: str, author: str = "owner") -> dict:
        record = operation_log.retraction(
            record_id=self.next_record_id(),
            recorded_at=now(),
            author=author,
            retracts=retracts,
            reason=reason,
        )
        # Resolving the whole log first is what refuses a retraction that names
        # nothing — before it is written, rather than after it has confused a
        # reader.
        operation_log.effective(
            [*self.operations(), record], editable_member=self.candidate_member
        )
        self.append(record)
        return record

    def staged(self) -> list[dict]:
        """The effective staged set: everything still standing, in log order."""
        return operation_log.effective(
            self.operations(), editable_member=self.candidate_member
        )

    def next_apply_id(self) -> str:
        directory = self.directory / APPLY_DIR
        existing = sorted(directory.glob("apply-*")) if directory.is_dir() else []
        return f"apply-{len(existing) + 1:04d}"

    def record_apply(self, apply_id: str, outcome: str, record: dict) -> dict:
        """Note in the log that an Apply happened, and where its record is."""
        entry = {
            "schema_version": SCHEMA_VERSION,
            "kind": operation_log.APPLY_RECORDED,
            "record_id": self.next_record_id(),
            "recorded_at": now(),
            "author": record.get("author", "owner"),
            "selection_id": None,
            "apply_id": apply_id,
            "outcome": outcome,
            "record": record["path"],
            "operation": None,
        }
        self.append(entry)
        return entry

    def record_candidate_acceptance(
        self, *, candidate_sha256: str, apply_id: str, note: str, author: str = "owner"
    ) -> dict:
        """The owner said yes — as a TYPED INTENT, and nothing more.

        Lifecycle state 3 is "owner-accepted editable master", and reaching it
        costs a tracked write, a re-signed receipt, and a changed digest
        constant in reviewed Rust source. None of those is the Workbench's to
        make, so what is recorded here is the intent: this session, this
        candidate digest, this Apply. The ceremony that acts on it is the
        owner's, outside this tool, and the record is what they carry into it.
        """
        record = {
            "schema_version": SCHEMA_VERSION,
            "kind": operation_log.CANDIDATE_ACCEPTED,
            "record_id": self.next_record_id(),
            "recorded_at": now(),
            "author": author,
            "selection_id": None,
            "candidate_sha256": candidate_sha256,
            "apply_id": apply_id,
            "note": note,
            "grants": {
                "tracked_write": False,
                "promotion": False,
                "receipt_resigned": False,
                "digest_constant_changed": False,
            },
            "ceremony": (
                "an accepted candidate becomes an accepted master only by the "
                "owner ceremony: the tracked write, promotion.json re-signed, and "
                "MASTER_DIGEST changed in reviewed source, together, in one commit"
            ),
            "operation": None,
        }
        self.append(record)
        return record

    def operations(self) -> list[dict]:
        path = self.directory / OPERATIONS_NAME
        if not path.is_file():
            return []
        return [
            json.loads(line)
            for line in path.read_text(encoding="utf-8").splitlines()
            if line.strip()
        ]


def open_session(projection: Projection, session_id: str | None = None) -> Session:
    """Create a session bound to the digests the projection was loaded at."""
    root = projection.root
    identifier = session_id or new_session_id()
    directory = root / SESSION_ROOT / identifier
    directory.mkdir(parents=True, exist_ok=True)
    (directory / SELECTIONS_DIR).mkdir(exist_ok=True)
    (directory / MASKS_DIR).mkdir(exist_ok=True)
    manifest = {
        "schema_version": SCHEMA_VERSION,
        "kind": MANIFEST_KIND,
        "workbench_version": VERSION,
        "session_id": identifier,
        "opened_at": now(),
        "view": "logical",
        "land_id": projection.land_id,
        "realm_id": projection.realm_id,
        "candidate_member": projection.candidate_member,
        "repository_revision": repository_revision(root),
        "revision_binding": "advisory",
        "digest_binding": "fail_closed",
        "projection": {"path": projection.path, "sha256": projection.digest},
        "base_digests": projection.source_records(),
        "authority": {
            # What a session may do, stated in the session itself. Staging and
            # Apply are V1 capabilities; neither is authority over anything
            # tracked, and the last two lines are the ones that matter.
            "staged_operations": True,
            "apply": True,
            "tracked_content": False,
            "runtime_input": False,
            "promotion": False,
        },
        "retention": {"policy": "disposable", "statement": RETENTION_STATEMENT},
    }
    _write(directory / MANIFEST_NAME, _json_bytes(manifest))
    return Session(root=root, directory=directory, session_id=identifier, manifest=manifest)


def prunable(
    root: Path,
    *,
    keep: str | None = None,
    days: int = RETENTION_DAYS,
    ceiling: int = RETENTION_KEEP,
    at: float | None = None,
) -> list[Path]:
    """Which session directories the retention ruling would remove.

    Separated from the removal so the decision is testable without deleting
    anything, and so a caller can print what it is about to do. Newest first by
    modification time; `keep` is the session currently in use and is never
    listed, however old it is.
    """
    sessions_root = Path(root) / SESSION_ROOT
    if not sessions_root.is_dir():
        return []
    entries = sorted(
        (path for path in sessions_root.iterdir() if path.is_dir()),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    moment = time.time() if at is None else at
    cutoff = moment - days * 86_400
    doomed: list[Path] = []
    survivors = 0
    for path in entries:
        if keep is not None and path.name == keep:
            survivors += 1
            continue
        if path.stat().st_mtime < cutoff or survivors >= ceiling:
            doomed.append(path)
        else:
            survivors += 1
    return doomed


def prune(
    root: Path,
    *,
    keep: str | None = None,
    days: int = RETENTION_DAYS,
    ceiling: int = RETENTION_KEEP,
    at: float | None = None,
) -> list[Path]:
    """Apply the retention ruling. Returns what was removed."""
    removed = prunable(root, keep=keep, days=days, ceiling=ceiling, at=at)
    for path in removed:
        shutil.rmtree(path, ignore_errors=True)
    return removed


def attach(root: Path, directory: Path) -> Session:
    """Open an existing session directory as it stands on disk."""
    directory = Path(directory)
    try:
        manifest = json.loads((directory / MANIFEST_NAME).read_bytes())
    except OSError as error:
        raise WorkbenchError(f"{directory} is not a session directory: {error}") from error
    if manifest.get("kind") != MANIFEST_KIND:
        raise WorkbenchError(f"{directory} does not carry a session manifest")
    return Session(
        root=Path(root),
        directory=directory,
        session_id=str(manifest.get("session_id", directory.name)),
        manifest=manifest,
    )
