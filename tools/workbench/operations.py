"""The staged-operation log: one record type, two authors.

The owner drags a building one cell east. An agent proposes the same edit from a
task description. Both write the same record, into the same file, in the same
order, distinguished only by `author`. That is not a nicety — it is what makes
agent parity mechanical rather than aspirational, and it is why this module
knows nothing about browsers.

**One log, appended to, never rewritten.** V0 established `operations.jsonl` as
the session's record of what happened, with an `operation` field that was always
null. V1 fills that field and adds nothing beside it: a staged operation is the
same envelope carrying a class, a verb, and parameters. Retracting a staged
operation appends a retraction naming it rather than deleting a line, because the
log is the honest record of a working session — what was tried and dropped is
part of what happened, and a file that is only ever appended to is a file two
processes can read without agreeing on anything.

**The effective set is a derivation, not a second list.** Apply and preview both
replay `effective(records, ...)`: every staged operation, in log order, minus the ones
a later record retracts. There is one such derivation and both callers use it, so
"what would Apply do" and "what did the preview show" cannot disagree.

**What this module deliberately does not know.** It never inspects a truth
operation's parameters. Those belong to the vocabulary's owner — the authoring
compiler, which declares the verbs, parses the parameters, and judges the result
(`crates/tme-authoring/src/operations.rs`). This module owns the envelope: who
staged it, which selection it derives from, which class it belongs to. Splitting
it there is what keeps one owner per fact; a Python copy of the parameter shapes
would be a table to keep equal, and a table to keep equal is a table that drifts.
"""

from __future__ import annotations

from .projection import WorkbenchError

SCHEMA_VERSION = 1

OPERATION_STAGED = "operation_staged"
OPERATION_RETRACTED = "operation_retracted"
APPLY_RECORDED = "apply_recorded"
CANDIDATE_ACCEPTED = "candidate_accepted"

#: The three operation classes of the design spec (§6.2). They are distinct
#: because their targets are distinct, and Apply keeps their routing separate
#: and their acceptance joint.
CLASS_TRUTH = "truth"
CLASS_DRESSING = "dressing"
CLASS_ASSET = "asset"
CLASSES = (CLASS_TRUTH, CLASS_DRESSING, CLASS_ASSET)

#: The dressing class exists in the vocabulary and ships zero verbs. That is a
#: bound limit, not an omission: a dressing edit changes visual presentation
#: without changing gameplay truth, and this project has accepted no
#: presentation fact to change. The only presentation the tree carries is a
#: diagnostic lattice whose per-class colour is DERIVED from the projection
#: rather than authored (`docs/presentation-direction.md`), so there is nothing
#: an owner could dress and nothing a "downward honest" check could bind
#: against. The rule is specified and tested; the verb set is empty until the
#: pixel grammar's accepted micro-scene binds a real target.
DRESSING_RULING = (
    "no dressing verb exists: this project has accepted no authored "
    "presentation fact, so there is nothing to dress. The class is reserved "
    "and its downward-honest rule is specified; a dressing verb arrives with "
    "the accepted micro-scene that gives it a target."
)


class OperationRefused(WorkbenchError):
    """A record cannot be staged or resolved, and the reason names what is wrong."""


def build(
    *,
    record_id: str,
    recorded_at: str,
    author: str,
    selection_id: str,
    operation_class: str,
    member: str,
    editable_member: str,
    verb: str,
    parameters: dict,
    adapter: dict | None = None,
    comment: str = "",
) -> dict:
    """Assemble one staged-operation record.

    `selection_id` is required. An operation is an act upon an address, and the
    packet is the address: without one there is no bound set of digests to
    re-verify at Apply, and the shared-pointing product requirement degrades
    into an agent asserting coordinates it typed. An agent that has no packet
    writes one first — `stage.py point` does exactly that.
    """
    record = {
        "schema_version": SCHEMA_VERSION,
        "kind": OPERATION_STAGED,
        "record_id": record_id,
        "recorded_at": recorded_at,
        "author": author,
        "selection_id": selection_id,
        "operation": {
            "class": operation_class,
            "member": member,
            "verb": verb,
            "parameters": parameters,
            # The typed adapter block of spec §9. Adapter-specific parameters
            # live here and are never smuggled into the shared fields above.
            "adapter": adapter,
        },
        "comment": comment,
    }
    validate(record, editable_member=editable_member)
    return record


def validate(record: dict, *, editable_member: str) -> None:
    """Refuse a staged record whose envelope is not the shape everything reads.

    The parameters are not inspected. What is checked here is what this module
    owns, and every check refuses rather than repairs.

    `editable_member` is passed in rather than known here. Which member has a
    candidate entry point is the land contract's fact, carried to this package
    in the compiler's own projection; a constant here would be a second opinion
    that is right about one land and wrong about the next.
    """
    if record.get("kind") != OPERATION_STAGED:
        raise OperationRefused(f"a staged operation carries kind {OPERATION_STAGED!r}")
    for field in ("record_id", "recorded_at", "author", "selection_id"):
        value = record.get(field)
        if not isinstance(value, str) or not value.strip():
            raise OperationRefused(f"a staged operation needs a non-empty {field}")
    operation = record.get("operation")
    if not isinstance(operation, dict):
        raise OperationRefused("a staged operation carries an operation object")
    operation_class = operation.get("class")
    if operation_class not in CLASSES:
        raise OperationRefused(
            f"unknown operation class {operation_class!r}; the classes are {CLASSES}"
        )
    if operation_class == CLASS_DRESSING:
        raise OperationRefused(DRESSING_RULING)
    verb = operation.get("verb")
    if not isinstance(verb, str) or not verb.strip():
        raise OperationRefused("a staged operation names a verb")
    if not isinstance(operation.get("parameters"), dict):
        raise OperationRefused(f"{verb}: parameters must be an object")
    adapter = operation.get("adapter", None)
    if adapter is not None and not isinstance(adapter, dict):
        raise OperationRefused(f"{verb}: the adapter block must be an object or null")
    if operation_class == CLASS_TRUTH and operation.get("member") != editable_member:
        raise OperationRefused(
            f"{verb}: truth operations address the {editable_member!r} member; "
            f"{operation.get('member')!r} has no candidate entry point"
        )


def retraction(*, record_id: str, recorded_at: str, author: str, retracts: str, reason: str) -> dict:
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": OPERATION_RETRACTED,
        "record_id": record_id,
        "recorded_at": recorded_at,
        "author": author,
        "selection_id": None,
        "retracts": retracts,
        "reason": reason,
        "operation": None,
    }


def effective(records, *, editable_member: str) -> list[dict]:
    """Every staged operation still standing, in log order.

    A retraction that names nothing, or names something already retracted, is a
    refusal rather than a no-op: a log that quietly ignored an instruction would
    be a log that does not describe the session.
    """
    staged: dict[str, dict] = {}
    order: list[str] = []
    retracted: set[str] = set()
    for record in records:
        kind = record.get("kind")
        if kind == OPERATION_STAGED:
            validate(record, editable_member=editable_member)
            identifier = record["record_id"]
            if identifier in staged:
                raise OperationRefused(f"the log stages {identifier} twice")
            staged[identifier] = record
            order.append(identifier)
        elif kind == OPERATION_RETRACTED:
            target = record.get("retracts")
            if target not in staged:
                raise OperationRefused(
                    f"{record.get('record_id')} retracts {target!r}, "
                    "which this log never staged"
                )
            if target in retracted:
                raise OperationRefused(
                    f"{record.get('record_id')} retracts {target!r}, "
                    "which was already retracted"
                )
            retracted.add(target)
    return [staged[identifier] for identifier in order if identifier not in retracted]


def by_class(operations, operation_class: str) -> list[dict]:
    return [record for record in operations if record["operation"]["class"] == operation_class]


def truth_operation_set(operations) -> dict:
    """The document the compiler's `replay` reads.

    Exactly the fields the compiler's own `StagedOperation` declares, and it
    refuses unknown ones — so this shape is checked on the other side of the
    bridge rather than trusted on this one.
    """
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": "workbench_truth_operation_set",
        "operations": [
            {
                "record_id": record["record_id"],
                "author": record["author"],
                "class": record["operation"]["class"],
                "member": record["operation"]["member"],
                "verb": record["operation"]["verb"],
                "parameters": record["operation"]["parameters"],
            }
            for record in by_class(operations, CLASS_TRUTH)
        ],
    }


def summary(operations) -> list[dict]:
    """The staged set as the interface and the receipts show it."""
    return [
        {
            "record_id": record["record_id"],
            "author": record["author"],
            "selection_id": record["selection_id"],
            "class": record["operation"]["class"],
            "member": record["operation"]["member"],
            "verb": record["operation"]["verb"],
            "parameters": record["operation"]["parameters"],
            "adapter": record["operation"]["adapter"],
            "comment": record.get("comment", ""),
        }
        for record in operations
    ]
