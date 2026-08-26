"""Adapters, and the registry that holds them at arm's length.

An adapter is the narrowest thing this layer knows how to talk to: hand it a
context image and a parameter block, get an image of the same size back. That is
the whole protocol, and the narrowness is deliberate. **An adapter is never
architecture authority.** It does not define the operation set, it does not own
the candidate lifecycle, it does not decide what gets promoted, and swapping one
out changes nothing above it. Anything an adapter needs that the shared fields
do not carry rides in its own typed block, never smuggled into a field the rest
of the layer reads.

**An adapter is not told about the commit mask, and that is the point.** The
project restores everything outside the mask itself, after the adapter returns
(`preserve.py`). An adapter that was handed the mask and honoured it would prove
only that a cooperative adapter cooperates — which is exactly the claim the
project cannot afford to rest on, because a hosted model is not cooperative, it
is a black box that returns pixels. So the reference adapter here is given a
crop and paints all of it, and the layer's proof is that painting all of it
changes nothing outside the mask.

**One adapter, and it is local.** `palette_fill` is deterministic arithmetic
over the standard library: no network, no model, no process, no hosted service.
The four hard limits of this package hold here in full. A hosted adapter is a
separate slice with its own credential handling, its own cost accounting, and
its own owner decision; nothing in this file anticipates it beyond the `kind`
field that says which sort of adapter a contract will accept.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass

from ..projection import WorkbenchError
from .png import Image

#: An adapter that computes its answer here, from its parameters alone, and
#: gives the same answer every time.
LOCAL_DETERMINISTIC = "local_deterministic"

#: An adapter that asks a generative model. None is registered in this slice.
GENERATIVE = "generative"

SOLID = "solid"
CHECKER = "checker"
PATTERNS = (SOLID, CHECKER)

DEFAULT_PERIOD = 8


class AdapterRefused(WorkbenchError):
    """An adapter cannot serve the call, and the reason names what is wrong."""


@dataclass(frozen=True)
class Adapter:
    """One registered adapter: its name, its sort, the verbs it serves, its work.

    `kind` and `verbs` are two different facts and both are checked. A contract
    says which *sorts* of provider it will accept — a policy the project sets
    once per operation. An adapter says which *operations* it is registered to
    serve — a claim about this particular implementation. Keeping them apart is
    what lets a refusal say "nothing is registered for this verb" and have it be
    literally true rather than approximately true.
    """

    name: str
    kind: str
    verbs: tuple[str, ...]
    summary: str
    apply: Callable[[Image, dict], Image]


def _colour(parameters: dict, key: str) -> tuple[int, int, int, int]:
    """One RGBA colour, or a refusal naming exactly how it is malformed."""
    try:
        value = parameters[key]
    except KeyError:
        raise AdapterRefused(f"palette_fill needs a {key!r} of four values, r g b a") from None
    if not isinstance(value, (list, tuple)) or len(value) != 4:
        raise AdapterRefused(
            f"palette_fill's {key!r} is {value!r}; it must be four values, r g b a"
        )
    channels = []
    for channel in value:
        if not isinstance(channel, int) or isinstance(channel, bool) or not 0 <= channel <= 255:
            raise AdapterRefused(
                f"palette_fill's {key!r} carries {channel!r}; each channel is a whole "
                "number from 0 to 255"
            )
        channels.append(int(channel))
    return tuple(channels)


def palette_fill(context: Image, parameters: dict) -> Image:
    """Paint the whole context image one colour, or a checker of two.

    It fills **everything** it is handed. It has no idea where the crop sits in
    the source, no idea which pixels the owner authorised, and no way to find
    out — the parameters carry a colour and a pattern and nothing else. That
    makes it the honest stand-in for a hosted model: a thing that returns a
    whole picture and takes no responsibility for what it overwrote.

    The pattern's squares are laid out from the crop's own top-left corner, so
    the same crop and the same parameters always give the same bytes.
    """
    unknown = sorted(set(parameters) - {"colour", "alternate", "pattern", "period"})
    if unknown:
        raise AdapterRefused(
            f"palette_fill was given {unknown}; it reads 'colour', 'alternate', "
            "'pattern', and 'period'"
        )
    pattern = parameters.get("pattern", SOLID)
    if pattern not in PATTERNS:
        raise AdapterRefused(
            f"palette_fill was asked for pattern {pattern!r}; it draws {list(PATTERNS)}"
        )
    colour = _colour(parameters, "colour")

    if pattern == SOLID:
        for absent in ("alternate", "period"):
            if absent in parameters:
                raise AdapterRefused(
                    f"palette_fill was given {absent!r} with pattern 'solid', where it "
                    "means nothing; a parameter that is ignored is a parameter a caller "
                    "believes took effect"
                )
        return context.with_pixels(bytes(colour) * (context.width * context.height))

    alternate = _colour(parameters, "alternate")
    period = parameters.get("period", DEFAULT_PERIOD)
    if not isinstance(period, int) or isinstance(period, bool) or period < 1:
        raise AdapterRefused(
            f"palette_fill was asked for period {period!r}; a checker square is at "
            "least one pixel across"
        )
    first, second = bytes(colour), bytes(alternate)
    pixels = bytearray()
    for y in range(context.height):
        band = (y // period) & 1
        pixels += b"".join(
            first if ((x // period) & 1) == band else second for x in range(context.width)
        )
    return context.with_pixels(bytes(pixels))


#: The one operation the reference adapter is registered to serve. Named as a
#: string rather than imported from `contracts`, because `contracts` imports
#: this module and an adapter is the lower layer of the two.
EDIT_REGION = "edit_region"

PALETTE_FILL = Adapter(
    name="palette_fill",
    kind=LOCAL_DETERMINISTIC,
    verbs=(EDIT_REGION,),
    summary="fill the whole context image with a colour, or a two-colour checker",
    apply=palette_fill,
)

#: Every adapter this slice registers. One, and it is local.
REGISTRY: dict[str, Adapter] = {PALETTE_FILL.name: PALETTE_FILL}


def lookup(name: str, registry: dict[str, Adapter] | None = None) -> Adapter:
    """The named adapter, or a refusal naming the whole registered set.

    The registry is injectable so that a proof can register a **hostile**
    adapter — one that scribbles over the entire context — without this file
    growing a test-only entry. A preservation rule proven only against the
    adapters the project ships is a rule proven against friends.
    """
    table = REGISTRY if registry is None else registry
    try:
        return table[name]
    except KeyError:
        raise AdapterRefused(
            f"no adapter named {name!r} is registered; this registry holds "
            f"{sorted(table)}"
        ) from None
