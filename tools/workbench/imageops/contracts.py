"""The five image operations, as typed records the project owns.

Spec section 9 names five operations, and this module is their contract. The
vocabulary is closed: a verb outside it is refused with the whole list, because
"the adapter also supports X" is how an adapter starts defining the operation
set, and the operation set is the project's.

**The bound limit of this slice, stated as a limit.** `edit_region` has an
implemented adapter — the local `palette_fill` — and an executor. The other four
are declared contracts with no adapter registered anywhere in this tree. They
`validate` as records, so a caller can write one and have its shape checked, and
they refuse to execute, naming the fact that nothing serves them. That refusal is
the honest state of the slice: a verb that parsed and then quietly did nothing
would be worse than one that does not parse.

**Adapter parameters live in the adapter block.** Every operation carries at most
one `{"adapter": <name>, "parameters": {...}}`, and everything an adapter needs
that the shared fields do not already carry goes inside `parameters`. Nothing
adapter-specific is ever added to a shared field: the moment `source` or
`context` grows a key that only one provider reads, the layer stops being
provider-neutral and the next adapter arrives as a migration.

**The field vocabulary is exactly what each operation's own contract sentence
names**, and no wider. Widening it is the job of the slice that implements the
verb, which will know what it actually needs rather than guessing here.
"""

from __future__ import annotations

from dataclasses import dataclass

from ..projection import WorkbenchError
from . import adapters

EDIT_REGION = "edit_region"
GENERATE_ASSET = "generate_asset"
ANIMATE_ASSET = "animate_asset"
NORMALIZE_PIXEL_GRID = "normalize_pixel_grid"
COMPARE_CANDIDATES = "compare_candidates"

#: The one verb this slice can execute.
IMPLEMENTED = EDIT_REGION

#: Fields every operation carries, whatever its verb.
UNIVERSAL = ("verb", "author")


class OperationRefused(WorkbenchError):
    """An operation record is not one this project will act on, and the reason says why."""


@dataclass(frozen=True)
class SourceRef:
    """One addressed file and the digest the operation was written against.

    The same path-and-digest pair the rest of the Workbench binds with, for the
    same fail-closed reason: an operation written against a master that has since
    moved is refused, never re-aimed at whatever is there now.
    """

    path: str
    sha256: str

    def as_record(self) -> dict[str, str]:
        return {"path": self.path, "sha256": self.sha256}

    @classmethod
    def from_record(cls, field_name: str, record) -> "SourceRef":
        if not isinstance(record, dict) or set(record) != {"path", "sha256"}:
            raise OperationRefused(
                f"{field_name} is {record!r}; it must be a path and a sha256"
            )
        return cls(str(record["path"]), str(record["sha256"]))


@dataclass(frozen=True)
class Context:
    """How much of the source is handed to the adapter, as a margin in pixels.

    A generous context is *encouraged*: context is what makes an edit blend, and
    an adapter shown only the committed pixels has nothing to blend into. The
    margin is safe to be generous with precisely because it does not widen what
    may change — the commit mask does that, and only an owner moves it.
    """

    margin: int

    @classmethod
    def from_record(cls, record) -> "Context":
        if not isinstance(record, dict) or set(record) != {"margin"}:
            raise OperationRefused(
                f"context is {record!r}; it must be a margin in source pixels"
            )
        margin = record["margin"]
        if not isinstance(margin, int) or isinstance(margin, bool) or margin < 0:
            raise OperationRefused(
                f"context margin is {margin!r}; it is a whole number of pixels, zero or more"
            )
        return cls(margin=int(margin))

    def as_record(self) -> dict[str, int]:
        return {"margin": self.margin}


@dataclass(frozen=True)
class AdapterBlock:
    """The typed block an adapter's own parameters ride in, and nowhere else."""

    adapter: str
    parameters: dict

    @classmethod
    def from_record(cls, record) -> "AdapterBlock":
        if not isinstance(record, dict) or set(record) != {"adapter", "parameters"}:
            raise OperationRefused(
                f"adapter is {record!r}; it must be an adapter name and its parameters"
            )
        if not isinstance(record["parameters"], dict):
            raise OperationRefused(
                f"adapter parameters are {record['parameters']!r}; they must be a block"
            )
        return cls(adapter=str(record["adapter"]), parameters=dict(record["parameters"]))

    def as_record(self) -> dict:
        return {"adapter": self.adapter, "parameters": dict(self.parameters)}


@dataclass(frozen=True)
class AssetOperation:
    """One requested operation, parsed. Fields a verb does not use stay None.

    One record for all five verbs rather than five records, because the
    lifecycle above this layer — validate, execute, present, promote — is one
    lifecycle, and a shape that varies per verb would push that decision into
    every caller.
    """

    verb: str
    author: str
    source: SourceRef | None = None
    commit_mask: SourceRef | None = None
    context: Context | None = None
    adapter: AdapterBlock | None = None
    references: tuple[SourceRef, ...] = ()
    grammar: str | None = None
    descriptor: str | None = None

    def as_record(self) -> dict:
        record: dict = {"verb": self.verb, "author": self.author}
        if self.source is not None:
            record["source"] = self.source.as_record()
        if self.commit_mask is not None:
            record["commit_mask"] = self.commit_mask.as_record()
        if self.context is not None:
            record["context"] = self.context.as_record()
        if self.adapter is not None:
            record["adapter"] = self.adapter.as_record()
        if self.references:
            record["references"] = [reference.as_record() for reference in self.references]
        if self.grammar is not None:
            record["grammar"] = self.grammar
        if self.descriptor is not None:
            record["descriptor"] = self.descriptor
        return record


@dataclass(frozen=True)
class OperationContract:
    """What one verb is, what it requires, and what may serve it."""

    name: str
    summary: str
    required: tuple[str, ...]
    optional: tuple[str, ...] = ()
    adapter_required: bool = True
    adapter_kinds: tuple[str, ...] = ()

    @property
    def fields(self) -> tuple[str, ...]:
        return UNIVERSAL + self.required + self.optional


CONTRACTS: dict[str, OperationContract] = {
    EDIT_REGION: OperationContract(
        name=EDIT_REGION,
        summary=(
            "Replace pixels inside an exact commit mask, given a wider context image."
        ),
        required=("source", "commit_mask", "context", "adapter"),
        adapter_required=True,
        adapter_kinds=(adapters.LOCAL_DETERMINISTIC, adapters.GENERATIVE),
    ),
    GENERATE_ASSET: OperationContract(
        name=GENERATE_ASSET,
        summary=(
            "Produce a new candidate asset against a stated grammar and reference set."
        ),
        required=("grammar", "references"),
        optional=("adapter",),
        adapter_required=True,
        adapter_kinds=(adapters.GENERATIVE,),
    ),
    ANIMATE_ASSET: OperationContract(
        name=ANIMATE_ASSET,
        summary="Produce candidate frames for an existing asset.",
        required=("source",),
        optional=("adapter",),
        adapter_required=True,
        adapter_kinds=(adapters.GENERATIVE,),
    ),
    NORMALIZE_PIXEL_GRID: OperationContract(
        name=NORMALIZE_PIXEL_GRID,
        summary=(
            "Force a candidate onto the project's pixel grid, palette discipline, "
            "and pivot conventions."
        ),
        required=("source", "grammar"),
        optional=("adapter",),
        adapter_required=True,
        adapter_kinds=(adapters.LOCAL_DETERMINISTIC,),
    ),
    COMPARE_CANDIDATES: OperationContract(
        name=COMPARE_CANDIDATES,
        summary=(
            "Present candidates against each other and against accepted masters "
            "under one descriptor."
        ),
        required=("references", "descriptor"),
        # The block is named in the vocabulary so that a record carrying one is
        # refused with the policy — this verb accepts no adapter — rather than
        # with a shape complaint about an unknown field. Presenting work for
        # judgement is the project's own act: no model decides what an owner is
        # shown beside what, so no adapter kind serves this verb.
        optional=("adapter",),
        adapter_required=False,
        adapter_kinds=(),
    ),
}

VOCABULARY = tuple(CONTRACTS)


def contract(verb: str) -> OperationContract:
    try:
        return CONTRACTS[verb]
    except KeyError:
        raise OperationRefused(
            f"unknown operation {verb!r}; this project defines {list(VOCABULARY)}"
        ) from None


def no_executor(verb: str) -> str:
    """The honest refusal for a declared verb this slice cannot perform."""
    return (
        f"no adapter is registered for {verb}; this slice implements "
        f"{adapters.PALETTE_FILL.name} for {IMPLEMENTED} alone"
    )


def _references(value) -> tuple[SourceRef, ...]:
    if not isinstance(value, list) or not value:
        raise OperationRefused(
            f"references is {value!r}; it must be a non-empty list of path-and-digest records"
        )
    return tuple(
        SourceRef.from_record(f"references[{index}]", record)
        for index, record in enumerate(value)
    )


def _text(field_name: str, value) -> str:
    if not isinstance(value, str) or not value.strip():
        raise OperationRefused(f"{field_name} is {value!r}; it must be a non-empty string")
    return value


def validate(operation: dict, *, registry: dict | None = None) -> AssetOperation:
    """Parse one operation record against its contract, or refuse naming the fault.

    Four refusals, and none of them is recoverable by guessing: an unknown verb,
    a missing required field, a field the verb does not define, and an adapter
    the contract will not accept. The last one checks the adapter's **kind** and
    the verbs it is registered for, not merely that the name resolves, so a local
    deterministic adapter cannot be handed a verb that needs a generative one — a
    call that would otherwise succeed and return something nobody asked for.

    Whether a verb *needs* an adapter is checked at execution, not here. The four
    unimplemented verbs have nothing registered to serve them, so requiring a
    block would make their records unwritable and hide the real state of the
    slice behind a shape error. A record is validated for its shape; whether
    anything can act on it is `run.execute`'s answer, and it is a plain one.
    """
    if not isinstance(operation, dict):
        raise OperationRefused(f"an operation is a record, and this is {operation!r}")
    if "verb" not in operation:
        raise OperationRefused(
            f"the operation names no verb; this project defines {list(VOCABULARY)}"
        )
    declared = contract(str(operation["verb"]))

    missing = [name for name in UNIVERSAL + declared.required if name not in operation]
    if missing:
        raise OperationRefused(
            f"{declared.name} is missing {missing}; it requires {list(declared.fields)}"
        )
    unknown = sorted(set(operation) - set(declared.fields))
    if unknown:
        raise OperationRefused(
            f"{declared.name} was given {unknown}, which it does not define; it reads "
            f"{list(declared.fields)}"
        )

    adapter_block = None
    if "adapter" in operation:
        adapter_block = AdapterBlock.from_record(operation["adapter"])
        if not declared.adapter_kinds:
            raise OperationRefused(
                f"{declared.name} accepts no adapter, and this record names "
                f"{adapter_block.adapter!r}"
            )
        found = adapters.lookup(adapter_block.adapter, registry)
        if found.kind not in declared.adapter_kinds:
            raise OperationRefused(
                f"adapter {found.name!r} is {found.kind} and {declared.name} accepts "
                f"{list(declared.adapter_kinds)}"
            )
        if declared.name not in found.verbs:
            raise OperationRefused(
                f"adapter {found.name!r} is registered for {list(found.verbs)} and this "
                f"operation is {declared.name}"
            )

    return AssetOperation(
        verb=declared.name,
        author=_text("author", operation["author"]),
        source=(
            SourceRef.from_record("source", operation["source"])
            if "source" in operation
            else None
        ),
        commit_mask=(
            SourceRef.from_record("commit_mask", operation["commit_mask"])
            if "commit_mask" in operation
            else None
        ),
        context=Context.from_record(operation["context"]) if "context" in operation else None,
        adapter=adapter_block,
        references=_references(operation["references"]) if "references" in operation else (),
        grammar=_text("grammar", operation["grammar"]) if "grammar" in operation else None,
        descriptor=(
            _text("descriptor", operation["descriptor"]) if "descriptor" in operation else None
        ),
    )
