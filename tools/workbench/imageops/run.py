"""Executing one `edit_region`, end to end, and writing nothing.

The order is fixed and every step is a refusal point: verify the source's
digest, verify the commit mask's digest, crop the context, call the adapter,
composite project-side, and reject the whole operation if the adapter wrote
outside the mask. Nothing later in the sequence runs on an input an earlier step
would not vouch for.

**Staleness is fail-closed here exactly as it is everywhere else in this tree.**
The source and the commit mask are each bound by path and SHA-256, each verified
through `projection.verify`, and each its own mutant: an operation written
against a master that has since moved is refused naming the moved digest, never
re-aimed at whatever is on disk now. An edit that follows a moved master is a
precise instruction turned into a confident wrong one.

**It writes no file, and that is a boundary, not an omission.** This package
composites pixels; it does not own the candidate directory, the session layout,
the promotion decision, or any other question about where bytes land. It returns
the encoded result and its digest, and the caller — which does own those
questions — decides. A layer that both produced candidates and filed them would
be the layer that decides what gets kept, and that decision is an owner's.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from ..projection import Source, WorkbenchError, digest_bytes, verify
from . import adapters, contracts, png, preserve
from .contracts import AssetOperation
from .masks import read_mask
from .preserve import Region

#: The roles the two verified inputs bind under, in the same vocabulary the
#: selection packet uses for its own bound sources.
SOURCE_ROLE = "edit_source"
COMMIT_MASK_ROLE = "commit_mask"


class EditRefused(WorkbenchError):
    """An edit will not be performed, and the reason names what stopped it."""


@dataclass(frozen=True)
class EditResult:
    """One completed edit: the bytes, what changed, and everything it read."""

    verb: str
    image: bytes
    sha256: str
    width: int
    height: int
    changed_pixels: int
    #: What the adapter changed outside the commit mask and this step discarded.
    #: Provenance, not a verdict — see `preserve`.
    outside_writes: int
    first_outside: tuple
    region: Region
    adapter: str
    parameters: dict
    read_digests: tuple[dict[str, str], ...]

    def as_record(self) -> dict:
        return {
            "verb": self.verb,
            "result": {"sha256": self.sha256, "width": self.width, "height": self.height},
            "changed_pixels": self.changed_pixels,
            "adapter_wrote_outside_the_mask": {
                "pixels": self.outside_writes,
                "first": [list(pixel) for pixel in self.first_outside],
                "note": (
                    "discarded by the project's compositing step; recorded because "
                    "what an adapter touched is provenance, not because it is a fault"
                ),
            },
            "context_region": self.region.as_record(),
            "adapter": {"adapter": self.adapter, "parameters": dict(self.parameters)},
            "read": [dict(record) for record in self.read_digests],
        }


def _verified_bytes(root: Path, role: str, reference) -> bytes:
    """One bound file, its digest checked before a single byte is used."""
    source = Source(role, reference.path, reference.sha256)
    verify(root, [source])  # raises StaleSelection naming the path and both digests
    return (Path(root) / reference.path).read_bytes()


def run_edit_region(
    operation: AssetOperation, *, root: Path, registry: dict | None = None
) -> EditResult:
    """Perform one `edit_region`, or refuse. Preservation is enforced here, project-side."""
    if operation.verb != contracts.EDIT_REGION:
        raise EditRefused(contracts.no_executor(operation.verb))
    if operation.source is None or operation.commit_mask is None:
        raise EditRefused("an edit_region names both a source and a commit mask")
    if operation.context is None or operation.adapter is None:
        raise EditRefused("an edit_region names both a context and an adapter")

    root = Path(root)
    image = png.decode(_verified_bytes(root, SOURCE_ROLE, operation.source))
    mask = read_mask(
        _verified_bytes(root, COMMIT_MASK_ROLE, operation.commit_mask),
        image_width=image.width,
        image_height=image.height,
    )

    region = preserve.context_region(image, mask, operation.context.margin)
    adapter = adapters.lookup(operation.adapter.adapter, registry)
    returned = adapter.apply(preserve.crop(image, region), dict(operation.adapter.parameters))
    if not isinstance(returned, png.Image):
        raise EditRefused(
            f"adapter {adapter.name!r} returned {type(returned).__name__}, and an adapter "
            "returns an image"
        )

    composited = preserve.composite(image, region, returned, mask)
    # What the adapter did outside the boundary is RECORDED, not refused: a
    # blending adapter returns a whole context image whose every pixel differs a
    # little, and there is no honest threshold that separates that from a
    # scribble. None of those pixels reached the result — that is the guarantee,
    # and it is checked immediately below rather than assumed.
    if not preserve.preserved_outside(image, composited.image, mask):
        # An independent check of the invariant the compositing step guarantees
        # by construction. If this ever fires, the guarantee is broken and the
        # only safe answer is to return nothing.
        raise EditRefused(
            "the composited result differs from the source outside the commit mask; "
            "the preservation step did not hold and no result is returned"
        )

    payload = png.encode(composited.image)
    return EditResult(
        outside_writes=composited.outside_writes,
        first_outside=composited.first_outside,
        verb=operation.verb,
        image=payload,
        sha256=digest_bytes(payload),
        width=composited.image.width,
        height=composited.image.height,
        changed_pixels=composited.changed_pixels,
        region=region,
        adapter=adapter.name,
        parameters=dict(operation.adapter.parameters),
        read_digests=(
            {"role": SOURCE_ROLE, **operation.source.as_record()},
            {"role": COMMIT_MASK_ROLE, **operation.commit_mask.as_record()},
        ),
    )


def execute(operation: AssetOperation, *, root: Path, registry: dict | None = None) -> EditResult:
    """The one execution entry point of this slice.

    `edit_region` is performed. The other four verbs are declared contracts with
    nothing registered to serve them, and saying so is the honest answer — a verb
    that validated and then returned an empty result would let a caller believe
    an asset had been produced.
    """
    if operation.verb != contracts.EDIT_REGION:
        raise EditRefused(contracts.no_executor(operation.verb))
    return run_edit_region(operation, root=root, registry=registry)
