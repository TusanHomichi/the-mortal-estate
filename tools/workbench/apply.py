#!/usr/bin/env python3
"""Apply: the only canonical emit, and it is atomic.

    python3 tools/workbench/apply.py .workbench/sessions/<id>
    python3 tools/workbench/apply.py <session> --json

Apply replays the complete staged set through the authoritative validators and
writes a candidate, a candidate projection, any edited assets, and a receipt.
Nothing else in this package writes an artifact that anyone is meant to look at
twice.

**The algorithm, in the order the safety depends on.**

1. Re-verify every bound digest — the session's base binding and every staged
   operation's selection packet. Any staleness aborts before anything is
   computed.
2. Replay the complete staged truth set, in log order, against a copy of the
   accepted master.
3. Judge that candidate with the compiler's own semantics, and judge every asset
   edit with the project's own preservation step.
4. On success: write the candidate, the candidate projection, the edited assets,
   and a receipt naming the base digests, the operation set, the outputs, and
   their digests.
5. On any failure: write the rejection record and nothing else.

**Atomicity is a rename, not a promise.** Everything is built inside a pending
directory that no reader is told about. Only when every step has passed does that
directory become `apply/apply-NNNN` in one `os.replace`. A failure removes it
entirely, so a rejected Apply leaves exactly one new file — the rejection record
— and no half-written candidate for anyone to mistake for an outcome. There is no
partial apply, no "apply the ones that passed", and no repair pass that silently
drops a failing operation.

**Apply is not promotion, and this is the distinction the whole safety story
rests on.** Everything Apply writes lands inside the disposable session directory
under the ignored working root. It writes no tracked content, edits no promotion
receipt, and changes no reviewed digest constant. Turning an accepted candidate
into an accepted master is an owner ceremony — a tracked write, `promotion.json`
re-signed, and `MASTER_DIGEST` changed in reviewed Rust source, together, in one
commit. The Workbench's job is to make the owner arrive at that ceremony already
certain, not to shorten it.

Exit codes: 0 applied, 2 rejected (the record says why), 3 the session could not
be read.
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from dataclasses import dataclass
from pathlib import Path

if __package__ in (None, ""):  # invoked as a script, the ordinary agent path
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from workbench import bridge  # noqa: E402
from workbench import imageops  # noqa: E402
from workbench import operations as operation_log  # noqa: E402
from workbench import replay as replay_module  # noqa: E402
from workbench.packet import now  # noqa: E402
from workbench.projection import WorkbenchError, digest_bytes  # noqa: E402
from workbench.resolve import find_root  # noqa: E402
from workbench.session import APPLY_DIR, Session, attach  # noqa: E402

EXIT_APPLIED = 0
EXIT_REJECTED = 2
EXIT_UNREADABLE = 3

RECEIPT_KIND = "workbench_apply_receipt"
REJECTION_KIND = "workbench_apply_rejection"
SCHEMA_VERSION = 1

STAGE_ASSET = "asset"

#: How many operations an attribution pass will replay to find the earliest one
#: the compiler will not accept. Linear and cheap for a taste round's handful;
#: past this the rejection stands unattributed rather than costing minutes.
ATTRIBUTION_CEILING = 16

APPLY_STATEMENT = (
    "Apply is not promotion. Everything named here lives inside a disposable "
    "session under the ignored working root. No tracked content was written, no "
    "promotion receipt was read or edited, and no reviewed digest constant was "
    "touched. An accepted candidate becomes an accepted master only by the owner "
    "ceremony that re-signs the receipt and changes the digest constant together."
)

REJECTION_STATEMENT = (
    "A rejected Apply writes this record and nothing else. No candidate, no "
    "projection, no asset, no partial result: the tree is exactly as it was."
)


@dataclass(frozen=True)
class Applied:
    """What one Apply attempt produced."""

    accepted: bool
    apply_id: str
    record: dict
    path: str


def apply(session: Session, *, author: str = "owner", registry: dict | None = None) -> Applied:
    """Run one Apply. Writes the receipt, or the rejection record, and nothing else."""
    apply_id = session.next_apply_id()
    pending = session.artifact(APPLY_DIR, f".pending-{apply_id}")
    shutil.rmtree(pending, ignore_errors=True)
    pending.mkdir(parents=True, exist_ok=True)
    try:
        outcome = replay_module.run(session, directory=pending, project=True)
        if not outcome.accepted:
            return _reject(
                session,
                apply_id,
                author,
                pending,
                stage=outcome.stage,
                assertion=_assertion(outcome),
                detail=outcome.as_record(),
                blamed=_blame(session, outcome, pending),
            )

        edits: list[dict] = []
        outputs = [_landed(outcome.candidate, apply_id, "candidate_master")]
        if outcome.projection:
            outputs.append(_landed(outcome.projection, apply_id, "candidate_projection"))

        for record in operation_log.by_class(outcome.operations, operation_log.CLASS_ASSET):
            try:
                result = _edit(session, record, registry)
            except WorkbenchError as refusal:
                return _reject(
                    session,
                    apply_id,
                    author,
                    pending,
                    stage=STAGE_ASSET,
                    assertion=str(refusal),
                    detail=outcome.as_record(),
                    blamed=record,
                )
            name = f"assets/{record['record_id']}.png"
            (pending / "assets").mkdir(exist_ok=True)
            # Written straight into the pending directory: nothing here is
            # visible until the whole attempt is, so there is nothing to make
            # atomic one file at a time.
            (pending / name).write_bytes(result.image)
            edits.append({"record_id": record["record_id"], **result.as_record()})
            outputs.append(
                {
                    "role": "candidate_asset",
                    "path": str(
                        (session.directory / APPLY_DIR / apply_id / name).relative_to(session.root)
                    ),
                    "sha256": result.sha256,
                }
            )

        receipt = {
            "schema_version": SCHEMA_VERSION,
            "kind": RECEIPT_KIND,
            "apply_id": apply_id,
            "applied_at": now(),
            "author": author,
            "session_id": session.session_id,
            "base": outcome.base,
            "bound_digests": session.manifest.get("base_digests", []),
            "operations": operation_log.summary(outcome.operations),
            "outputs": outputs,
            "candidate_report": outcome.detail,
            "asset_edits": edits,
            "grants": {
                "promotion": False,
                "tracked_write": False,
                "runtime_input": False,
                "receipt_resigned": False,
                "digest_constant_changed": False,
            },
            "statement": APPLY_STATEMENT,
        }
        payload = _bytes(receipt)
        (pending / "receipt.json").write_bytes(payload)

        # The one moment anything becomes visible under a name a reader is told
        # about. Everything above either completed or left nothing behind.
        final = session.artifact(APPLY_DIR, apply_id)
        shutil.rmtree(final, ignore_errors=True)
        os.replace(pending, final)
        record = {
            "path": str((final / "receipt.json").relative_to(session.root)),
            "sha256": digest_bytes(payload),
            "author": author,
        }
        session.record_apply(apply_id, "applied", record)
        return Applied(True, apply_id, receipt, record["path"])
    finally:
        shutil.rmtree(pending, ignore_errors=True)


def _bytes(document: dict) -> bytes:
    return (json.dumps(document, indent=2) + "\n").encode("utf-8")


def _landed(record: dict, apply_id: str, role: str) -> dict:
    """Name an output where it will live, not where it was built.

    Everything is built inside `apply/.pending-<id>` and becomes `apply/<id>` in
    one rename. A receipt that named the pending path would name a directory
    that no longer exists by the time anyone reads it.
    """
    built = f"{APPLY_DIR}/.pending-{apply_id}"
    landed = f"{APPLY_DIR}/{apply_id}"
    return {
        "role": role,
        "path": record["path"].replace(built, landed),
        "sha256": record["sha256"],
    }


def _assertion(outcome) -> str:
    """The refusal in the validator's own words, never paraphrased."""
    detail = outcome.detail
    if outcome.stage == replay_module.STAGE_STALENESS:
        return detail["error"]
    if outcome.stage == replay_module.STAGE_REPLAY:
        return detail.get("error", json.dumps(detail))
    diagnostics = detail.get("diagnostics") if isinstance(detail, dict) else None
    if diagnostics:
        return diagnostics[0]
    return json.dumps(detail)


def _blame(session: Session, outcome, pending: Path) -> dict | None:
    """Which operation the refusal belongs to, when that can be said honestly.

    A replay refusal names its record, because the replay knows which verb it
    was applying. A VALIDATOR refusal does not: the compiler judges the candidate
    document as a whole and has no idea which staged operation put the cell
    there. Rather than guess, this replays growing prefixes of the staged set and
    reports **the earliest prefix the compiler will not accept**.

    That is a narrower claim than "the culprit", and the record says so: a later
    operation can repair what an earlier one broke, so the earliest failing
    prefix is where the trouble starts, not necessarily the only place it lives.
    """
    if outcome.stage == replay_module.STAGE_REPLAY:
        text = outcome.detail.get("error", "")
        for record in outcome.operations:
            if record["record_id"] in text:
                return record
        return None
    if outcome.stage != replay_module.STAGE_VALIDATE:
        return None
    truth = operation_log.by_class(outcome.operations, operation_log.CLASS_TRUTH)
    if len(truth) <= 1 or len(truth) > ATTRIBUTION_CEILING:
        return truth[0] if len(truth) == 1 else None
    scratch = pending / "attribution"
    base = replay_module.base_binding(session)
    for index in range(1, len(truth) + 1):
        payload = operation_log.truth_operation_set(truth[:index])
        scratch.mkdir(parents=True, exist_ok=True)
        path = scratch / replay_module.OPERATIONS_NAME
        path.write_bytes(_bytes(payload))
        answer = bridge.replay(
            session.root,
            land=replay_module.land_of(session),
            operations=path,
            output_directory=scratch,
            expect_base_sha256=base["sha256"],
            validate=True,
        )
        if not answer.yes:
            return truth[index - 1]
    return None


def _edit(session: Session, record: dict, registry: dict | None):
    """One asset operation, validated against its contract and then performed."""
    operation = record["operation"]
    payload = {
        "verb": operation["verb"],
        "author": record["author"],
        **operation["parameters"],
    }
    if operation.get("adapter") is not None:
        payload["adapter"] = operation["adapter"]
    parsed = imageops.validate(payload, registry=registry)
    return imageops.execute(parsed, root=session.root, registry=registry)


def _reject(
    session: Session,
    apply_id: str,
    author: str,
    pending: Path,
    *,
    stage: str,
    assertion: str,
    detail: dict,
    blamed: dict | None,
) -> Applied:
    """Write the rejection record. Remove everything the attempt built."""
    shutil.rmtree(pending, ignore_errors=True)
    rejection = {
        "schema_version": SCHEMA_VERSION,
        "kind": REJECTION_KIND,
        "apply_id": apply_id,
        "rejected_at": now(),
        "author": author,
        "session_id": session.session_id,
        "stage": stage,
        "operation": operation_log.summary([blamed])[0] if blamed else None,
        "attribution": (
            "the replay named this operation"
            if stage == replay_module.STAGE_REPLAY and blamed
            else (
                "the earliest staged prefix the compiler will not accept; a later "
                "operation can repair what an earlier one broke, so this is where "
                "the trouble starts rather than necessarily the only place it lives"
                if stage == replay_module.STAGE_VALIDATE and blamed
                else "the refusal names no single operation"
            )
        ),
        "assertion": assertion,
        "detail": detail,
        "statement": REJECTION_STATEMENT,
    }
    payload = _bytes(rejection)
    path = session.artifact(APPLY_DIR, f"{apply_id}.rejection.json")
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)
    relative = str(path.relative_to(session.root))
    session.record_apply(
        apply_id, "rejected", {"path": relative, "sha256": digest_bytes(payload), "author": author}
    )
    return Applied(False, apply_id, rejection, relative)


def render(applied: Applied) -> str:
    record = applied.record
    if applied.accepted:
        lines = [
            f"APPLIED    {applied.apply_id}  receipt {applied.path}",
            f"base       {record['base']['sha256'][:12]}  {record['base']['path']}",
            f"operations {len(record['operations'])} staged, replayed in log order",
        ]
        lines.extend(
            f"  {entry['record_id']}  {entry['class']:<6} {entry['verb']}"
            for entry in record["operations"]
        )
        lines.append("outputs")
        lines.extend(
            f"  {entry['role']:<22} {entry['sha256'][:12]}  {entry['path']}"
            for entry in record["outputs"]
        )
        lines.append(f"promotion  NOT GRANTED — {APPLY_STATEMENT.splitlines()[0]}")
        return "\n".join(lines)
    lines = [
        f"REJECTED   {applied.apply_id}  record {applied.path}",
        f"stage      {record['stage']}",
    ]
    if record["operation"]:
        lines.append(
            f"operation  {record['operation']['record_id']}  "
            f"{record['operation']['verb']}   ({record['attribution']})"
        )
    lines.append("assertion  " + record["assertion"])
    lines.append("tree       unchanged")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("session", help="path to a session directory")
    parser.add_argument("--root", default=None, help="repository root (default: inferred)")
    parser.add_argument("--author", default="owner")
    parser.add_argument("--json", action="store_true", help="emit the record as JSON")
    arguments = parser.parse_args(argv)
    directory = Path(arguments.session).resolve()
    try:
        root = Path(arguments.root).resolve() if arguments.root else find_root(directory)
        session = attach(root, directory)
        applied = apply(session, author=arguments.author)
    except WorkbenchError as error:
        print(f"UNREADABLE: {error}", file=sys.stderr)
        return EXIT_UNREADABLE
    if arguments.json:
        print(json.dumps(applied.record, indent=2))
    else:
        print(render(applied))
    return EXIT_APPLIED if applied.accepted else EXIT_REJECTED


if __name__ == "__main__":
    sys.exit(main())
