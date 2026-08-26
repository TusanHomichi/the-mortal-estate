"""The project's own compositing step: the rule that keeps edits from eroding work.

Spec section 9, quoted because it is the load-bearing sentence of this package:

    The project owns preservation. A model may receive a generous context image,
    because context is what makes an edit blend. Only the exact commit mask may
    replace accepted source pixels. Everything outside the commit mask is
    restored from the source, byte-for-byte, by the project's own compositing
    step after the adapter returns — never by trusting the adapter to have left
    it alone. Expanding the commit boundary is an explicit owner act, recorded as
    its own operation. This is the single rule that keeps generative editing from
    quietly eroding accepted work, and it is enforced project-side precisely
    because no adapter can be trusted to enforce it.

**The invariant holds by construction, not by inspection.** `composite` starts
from a copy of the source and copies in only the mask's pixels. It never diffs
the adapter's output against the source and repairs what it finds wrong. The
difference matters: a compare-and-fix step is correct only if its comparison is
correct, so it converts a structural guarantee into a code-review question. Here
there is no path by which an out-of-mask byte from the adapter reaches the
result, because no line of this module ever reads one into it.

**Restoration is the rule; the count is provenance, not a verdict.** An adapter
that blends is *supposed* to return a whole context image whose every pixel
differs a little — that is what "context is what makes an edit blend" means. So
`outside_writes` is recorded, on the operation's receipt, as an observation of
what the adapter did; it is not a rejection, because there is no honest threshold
between a model that blended and a model that scribbled, and a rule that refused
both would refuse every generative adapter this contract exists to accommodate.
What protects the accepted work is that none of those pixels can reach the
result, whatever the adapter's intent was.

**What IS blocking is the invariant itself.** [`preserved_outside`] re-derives
it independently, over the whole image, and `run.run_edit_region` refuses to
return a result that fails it. That check is qualified by a mutant on THIS
module rather than on an adapter: a compositing step that trusts the adapter's
output wholesale is caught by it (`tests/test_workbench_imageops.py`). Qualifying
it against a hostile adapter alone would have proven only that a hostile adapter
loses, which the construction above already guarantees.
"""

from __future__ import annotations

from dataclasses import dataclass

from ..projection import WorkbenchError
from .masks import Mask
from .png import BYTES_PER_PIXEL, Image

#: How many offending coordinates a refusal carries. Enough to see the shape of
#: the violation, few enough that the message stays readable.
FIRST_OUTSIDE_REPORTED = 4


class PreservationRefused(WorkbenchError):
    """A composite cannot be performed as described, and the reason names why."""


@dataclass(frozen=True)
class Region:
    """A rectangle of one image, in that image's own pixels."""

    x: int
    y: int
    width: int
    height: int

    def contains(self, x: int, y: int) -> bool:
        return self.x <= x < self.x + self.width and self.y <= y < self.y + self.height

    def as_record(self) -> dict[str, int]:
        return {"x": self.x, "y": self.y, "width": self.width, "height": self.height}


def context_region(image: Image, mask: Mask, margin: int) -> Region:
    """The crop handed to the adapter: the mask's bounding box, grown and clipped.

    Grown by `margin` on all four sides because an adapter shown only the pixels
    it may replace has nothing to blend into, and clipped to the image because a
    crop is a crop of something. Both steps are pure arithmetic on the covered
    set, so the same mask and the same margin always name the same rectangle.
    """
    if not isinstance(margin, int) or isinstance(margin, bool) or margin < 0:
        raise PreservationRefused(
            f"a context margin of {margin!r} is not a whole number of pixels, zero or more"
        )
    if not mask.covered:
        raise PreservationRefused("an empty commit mask has no region to grow")
    box_x, box_y, box_width, box_height = mask.bounding_box()
    left = max(0, box_x - margin)
    top = max(0, box_y - margin)
    right = min(image.width, box_x + box_width + margin)
    bottom = min(image.height, box_y + box_height + margin)
    if right <= left or bottom <= top:
        raise PreservationRefused(
            f"the commit mask's box {box_x},{box_y} {box_width}x{box_height} lies outside "
            f"a {image.width}x{image.height} image"
        )
    return Region(x=left, y=top, width=right - left, height=bottom - top)


def crop(image: Image, region: Region) -> Image:
    """The region's pixels, as an image of its own."""
    if (
        region.x < 0
        or region.y < 0
        or region.x + region.width > image.width
        or region.y + region.height > image.height
    ):
        raise PreservationRefused(
            f"region {region.as_record()} does not fit inside a "
            f"{image.width}x{image.height} image"
        )
    stride = image.width * BYTES_PER_PIXEL
    span = region.width * BYTES_PER_PIXEL
    pixels = bytearray()
    for row in range(region.height):
        start = (region.y + row) * stride + region.x * BYTES_PER_PIXEL
        pixels += image.pixels[start : start + span]
    return Image(width=region.width, height=region.height, pixels=bytes(pixels))


@dataclass(frozen=True)
class Composited:
    """The safe result, and what the adapter did to earn it.

    `changed_pixels` is what the operation accomplished. `outside_writes` is what
    it attempted and was denied — pixels inside the context crop but outside the
    commit mask that came back different from the source. `first_outside` carries
    the first few of those coordinates so a refusal can point at one.
    """

    image: Image
    changed_pixels: int
    outside_writes: int
    first_outside: tuple[tuple[int, int], ...] = ()


def composite(source: Image, region: Region, returned: Image, mask: Mask) -> Composited:
    """Build the result from the source, then copy in the commit mask alone.

    The adapter's output is read at exactly the coordinates the mask covers, and
    at no others. Every remaining byte of the result is the source's own byte,
    unexamined and unmodified.
    """
    if (returned.width, returned.height) != (region.width, region.height):
        raise PreservationRefused(
            f"the adapter returned a {returned.width}x{returned.height} image and the "
            f"context region is {region.width}x{region.height}"
        )
    uncovered = sorted(
        pixel for pixel in mask.covered if not region.contains(pixel[0], pixel[1])
    )
    if uncovered:
        # Otherwise part of what the owner authorised would silently go
        # uncommitted, and the operation would report success for an edit it
        # only partly applied.
        raise PreservationRefused(
            f"the context region {region.as_record()} does not contain commit-mask pixel "
            f"{uncovered[0]}; {len(uncovered)} authorised pixels fall outside it"
        )

    result = bytearray(source.pixels)  # every byte from the source, to start with
    stride = source.width * BYTES_PER_PIXEL
    changed = 0
    outside = 0
    offenders: list[tuple[int, int]] = []
    for row in range(region.height):
        y = region.y + row
        for column in range(region.width):
            x = region.x + column
            given = (row * region.width + column) * BYTES_PER_PIXEL
            candidate = returned.pixels[given : given + BYTES_PER_PIXEL]
            target = y * stride + x * BYTES_PER_PIXEL
            original = source.pixels[target : target + BYTES_PER_PIXEL]
            if mask.covers(x, y):
                if candidate != original:
                    result[target : target + BYTES_PER_PIXEL] = candidate
                    changed += 1
            elif candidate != original:
                outside += 1
                if len(offenders) < FIRST_OUTSIDE_REPORTED:
                    offenders.append((x, y))
    return Composited(
        image=source.with_pixels(bytes(result)),
        changed_pixels=changed,
        outside_writes=outside,
        first_outside=tuple(offenders),
    )


def preserved_outside(source: Image, result: Image, mask: Mask) -> bool:
    """Whether every pixel outside the commit mask is byte-identical to the source.

    An independent verifier over the **whole** image, not the context region: it
    takes no argument from `composite` and shares none of its code, so a caller
    or a proof can assert the invariant without trusting the thing that produced
    the result. Rows the mask does not touch are compared as whole slices, which
    is why checking a full-size master costs a fraction of a second rather than a
    pass over every pixel in Python.
    """
    if (source.width, source.height) != (result.width, result.height):
        return False
    stride = source.width * BYTES_PER_PIXEL
    rows: dict[int, set[int]] = {}
    for x, y in mask.covered:
        rows.setdefault(y, set()).add(x)
    for y in range(source.height):
        start = y * stride
        stop = start + stride
        covered = rows.get(y)
        if not covered:
            if source.pixels[start:stop] != result.pixels[start:stop]:
                return False
            continue
        for x in range(source.width):
            if x in covered:
                continue
            offset = start + x * BYTES_PER_PIXEL
            if (
                source.pixels[offset : offset + BYTES_PER_PIXEL]
                != result.pixels[offset : offset + BYTES_PER_PIXEL]
            ):
                return False
    return True
