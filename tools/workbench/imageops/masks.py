"""The mask format, and the commit mask that decides what an edit may replace.

**One format, two address spaces, and the difference is load-bearing.** A
*selection* mask, written by `packet.mask_bytes`, addresses **cells** of the
authored lattice: it says which squares of the world an owner pointed at. A
*commit* mask, read here, addresses **pixels** of a source image: it says which
bytes of an accepted master an operation is permitted to replace. Same P1
portable bitmap, same origin comment, same decoder — different space. Nothing
in the file distinguishes them, so nothing but the caller can: a consumer that
hands a selection mask to `read_mask` gets a mask over pixels 0..n of the
picture, which is not what those numbers meant. Which space a mask is in
belongs to the operation that names it.

P1 was chosen for the selection mask because it needs no library to read, no
library to write, and diffs as text. The commit mask inherits all three, plus
the more important property: an owner can read one and see exactly what an edit
was allowed to touch.

The decoder lives here rather than in `resolve.py` because there is now more
than one reader of the format, and two decoders of one format are two answers
waiting to disagree. `resolve.py` imports this one.
"""

from __future__ import annotations

from dataclasses import dataclass

from ..projection import WorkbenchError

MAGIC = "P1"

#: The comment that carries the origin. A P1 bitmap has no place for one, so the
#: origin rides in a comment the format requires every reader to skip — which is
#: why a reader that skips comments blindly loses the mask's position entirely.
ORIGIN_PREFIX = "# origin "


class MaskUnreadable(WorkbenchError):
    """A mask is not the bitmap it claims to be, or claims nothing at all."""


@dataclass(frozen=True)
class Mask:
    """One decoded mask: a box at an origin, and the exact set it covers.

    `covered` holds absolute coordinates — the origin is already added — because
    every caller works in the source image's space and a reader that had to
    remember to add the origin itself would eventually forget.
    """

    origin_x: int
    origin_y: int
    width: int
    height: int
    covered: frozenset[tuple[int, int]]

    def covers(self, x: int, y: int) -> bool:
        return (x, y) in self.covered

    @property
    def count(self) -> int:
        return len(self.covered)

    def bounding_box(self) -> tuple[int, int, int, int]:
        """The tightest box around the covered pixels, as (x, y, width, height).

        Tighter than the declared box whenever the mask's outer rows or columns
        carry nothing. The covered set is what may be replaced, so the covered
        set is what the context region is grown from.
        """
        if not self.covered:
            raise MaskUnreadable("an empty mask has no bounding box")
        xs = [x for x, _ in self.covered]
        ys = [y for _, y in self.covered]
        return min(xs), min(ys), max(xs) - min(xs) + 1, max(ys) - min(ys) + 1


def decode_p1(payload: bytes) -> Mask:
    """Decode a P1 bitmap and the origin its comment records, in either space.

    Deliberately free of policy: it says what the bytes are, not whether they
    are an acceptable commit mask. `read_mask` layers that on, and `resolve.py`
    layers on the selection rules instead.
    """
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise MaskUnreadable(f"the mask is not UTF-8 text: {error}") from error

    origin_x = origin_y = 0
    tokens: list[str] = []
    for line in text.splitlines():
        if line.startswith("#"):
            if line.startswith(ORIGIN_PREFIX):
                coordinate = line.removeprefix(ORIGIN_PREFIX).split()[0]
                try:
                    origin_x, origin_y = (int(part) for part in coordinate.split(","))
                except ValueError as error:
                    raise MaskUnreadable(
                        f"the mask records origin {coordinate!r}, and an origin is x,y"
                    ) from error
            continue
        tokens.extend(line.split())

    if not tokens or tokens[0] != MAGIC:
        raise MaskUnreadable(f"the mask is not a {MAGIC} bitmap")
    if len(tokens) < 3:
        raise MaskUnreadable("the mask header ends before it declares a width and a height")
    try:
        width, height = int(tokens[1]), int(tokens[2])
    except ValueError as error:
        raise MaskUnreadable(
            f"the mask declares size {tokens[1]!r} by {tokens[2]!r}, which is not a size"
        ) from error
    if width <= 0 or height <= 0:
        raise MaskUnreadable(f"the mask declares {width}x{height}, which is not a region")

    bits = "".join(tokens[3:])
    if len(bits) != width * height:
        raise MaskUnreadable(
            f"the mask declares {width}x{height} and carries {len(bits)} bits"
        )
    unexpected = sorted(set(bits) - {"0", "1"})
    if unexpected:
        raise MaskUnreadable(
            f"the mask carries {unexpected}, and a {MAGIC} bitmap carries only 0 and 1"
        )

    covered = frozenset(
        (origin_x + index % width, origin_y + index // width)
        for index, bit in enumerate(bits)
        if bit == "1"
    )
    return Mask(
        origin_x=origin_x,
        origin_y=origin_y,
        width=width,
        height=height,
        covered=covered,
    )


def read_mask(payload: bytes, *, image_width: int, image_height: int) -> Mask:
    """A commit mask over a source image of the given size, or a refusal.

    Three refusals, and each one is a way an edit could otherwise go wrong:

    - a mask whose declared size disagrees with its bit count is not a region,
      it is a file someone edited by hand and did not finish;
    - an **empty** mask is refused because an operation that may replace nothing
      is not an operation, and accepting one would let a caller believe an edit
      was applied when the compositing step had nothing to copy;
    - a mask reaching **outside** the source image is refused rather than
      clipped, because clipping silently changes what the owner authorised. The
      caller passes the image's size in; the mask does not get to assume it.
    """
    mask = decode_p1(payload)
    if not mask.covered:
        raise MaskUnreadable(
            "the commit mask covers no pixel, and an operation that may replace "
            "nothing is not an operation"
        )
    outside = sorted(
        pixel
        for pixel in mask.covered
        if not (0 <= pixel[0] < image_width and 0 <= pixel[1] < image_height)
    )
    if outside:
        raise MaskUnreadable(
            f"the commit mask names pixel {outside[0]} and the source image is "
            f"{image_width}x{image_height}; {len(outside)} covered pixels fall outside it"
        )
    return mask


def write_mask(covered, *, note: str = "workbench commit mask, in source image pixels") -> bytes:
    """The exact covered set as a P1 bitmap over its own bounding box.

    The box is the tightest one around the covered pixels, so the file carries
    no rows that mean nothing. The note is a comment, and the origin is the
    comment that matters: without it the bitmap says which shape was committed
    but not where.
    """
    pixels = {(int(x), int(y)) for x, y in covered}
    if not pixels:
        raise MaskUnreadable("a mask covering no pixel cannot be written")
    origin_x = min(x for x, _ in pixels)
    origin_y = min(y for _, y in pixels)
    width = max(x for x, _ in pixels) - origin_x + 1
    height = max(y for _, y in pixels) - origin_y + 1
    rows = [
        "".join(
            "1" if (origin_x + dx, origin_y + dy) in pixels else "0"
            for dx in range(width)
        )
        for dy in range(height)
    ]
    header = (
        f"{MAGIC}\n"
        f"# {note}\n"
        f"{ORIGIN_PREFIX}{origin_x},{origin_y} in the source image's pixel lattice\n"
        f"{width} {height}\n"
    )
    return (header + "\n".join(rows) + "\n").encode("utf-8")
