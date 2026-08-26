"""The selection packet: the unit of pointing.

A packet is a typed record naming exactly what the owner pointed at, bound to
the exact bytes it was taken against. It is written as plain JSON into the
session directory so that an agent with ordinary file tools can read it without
running anything.

Two rules govern the shape and neither is negotiable. **The comment is the only
free-text field**, and nothing downstream may recover from it a fact another
field could have carried. **The binding is digests**, so a consumer can always
decide whether the packet still describes the tree in front of it.

**One record type, two views.** A logical selection and a capture selection are
the same record with different values, never different shapes. The fields a
capture fills — `camera`, `scene.frame_generation`, the capture's three bound
digests, `context_image`, and `observed` — are present and empty on a logical
packet, so a consumer written against one reads the other without a branch.

**`observed` is not `semantic`.** The semantic set names the authored world: the
structures, transitions, landmarks, routes, terrain, and layers an agent can act
on, resolved from the compiler's projection over the covered cells. That is why
a capture selection and a logical selection over the same region carry an
identical semantic set. What the frame happened to be showing at the instant of
the photograph — an actor mid-step, a corpse, a dropped coin — is real, is worth
recording, and is not an authored identity. It goes in `observed`, where a
consumer can see it without mistaking it for something addressable in the land.
"""

from __future__ import annotations

from datetime import datetime, timezone

from . import VERSION
from .identity import resolve
from .projection import Member, Projection, WorkbenchError

PACKET_KIND = "workbench_selection_packet"
SCHEMA_VERSION = 1
GESTURES = ("click", "box", "lasso", "paint")

VIEW_LOGICAL = "logical"
VIEW_CAPTURE = "capture"

#: The space a gesture's recorded geometry is expressed in. One packet type
#: carries both views, so the space is a field rather than an assumption.
SPACE_CELLS = "authored_cell_lattice"
SPACE_PIXELS = "capture_image_pixels"

#: Gestures whose covered set cannot be re-derived from a rectangle, and which
#: therefore carry a mask naming the exact cells.
MASKED_GESTURES = ("lasso", "paint")

#: What each gesture's geometry is made of, per view. The names differ because
#: the spaces differ: the logical view points at cells, a capture points at
#: pixels, and calling both "cell" would be the kind of quiet conflation that
#: puts a pixel coordinate into a cell field.
LOGICAL_GEOMETRY_KEYS = {
    "click": ("cell",),
    "box": ("rect",),
    "lasso": ("polygon",),
    "paint": ("cells",),
}
CAPTURE_GEOMETRY_KEYS = {
    "click": ("point",),
    "box": ("rect",),
    "lasso": ("polygon",),
    "paint": ("points",),
}


def geometry_of(gesture: str, body: dict, keys: dict | None = None) -> dict:
    """The gesture, extracted from a request exactly as it was made.

    Recorded on the packet so a consumer re-derives the address rather than
    trusting it. Nothing else from the request body may enter it: a geometry
    that carried a caller's own idea of which cells it covered would defeat the
    point of re-deriving them.
    """
    table = keys if keys is not None else LOGICAL_GEOMETRY_KEYS
    if gesture not in table:
        raise WorkbenchError(f"unknown gesture {gesture!r}; expected one of {tuple(table)}")
    geometry = {}
    for name in table[gesture]:
        if name not in body:
            raise WorkbenchError(f"a {gesture} selection needs {name!r}")
        geometry[name] = body[name]
    return geometry


def now() -> str:
    """A wall-clock stamp, for human reading only. Nothing resolves by time."""
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _point(value) -> tuple[int, int]:
    try:
        return int(value["x"]), int(value["y"])
    except (KeyError, TypeError, ValueError) as error:
        raise WorkbenchError(f"malformed cell reference: {value!r}") from error


def cells_for_gesture(member: Member, gesture: str, payload: dict) -> list[tuple[int, int]]:
    """The exact cells a gesture covers, derived here rather than trusted.

    Click and box are re-derived from the geometry the client reports, so a
    client that miscounts cannot write a wrong address. Paint is inherently a
    client-side path — the brush visited what it visited — so its cells arrive
    as data and are validated against the member's envelope instead.
    """
    if gesture not in GESTURES:
        raise WorkbenchError(f"unknown gesture {gesture!r}; expected one of {GESTURES}")

    if gesture == "click":
        cells = [_point(payload["cell"])]
    elif gesture == "box":
        rect = payload["rect"]
        origin_x, origin_y = int(rect["x"]), int(rect["y"])
        width, height = int(rect["width"]), int(rect["height"])
        if width <= 0 or height <= 0:
            raise WorkbenchError("a box selection must cover at least one cell")
        cells = [
            (origin_x + dx, origin_y + dy) for dy in range(height) for dx in range(width)
        ]
    elif gesture == "lasso":
        cells = _lasso_cells(member, payload["polygon"])
    else:
        cells = [_point(entry) for entry in payload["cells"]]

    if not cells:
        raise WorkbenchError("the gesture covered no cells")
    outside = [cell for cell in cells if not member.contains(cell)]
    if outside:
        raise WorkbenchError(
            f"the gesture covers {len(outside)} cell(s) outside member "
            f"{member.member!r}, first at {outside[0]}"
        )
    return sorted(set(cells), key=lambda cell: (cell[1], cell[0]))


def _lasso_cells(member: Member, polygon) -> list[tuple[int, int]]:
    """Cells whose centre falls inside the drawn polygon, by the even-odd rule.

    The polygon arrives in cell coordinates, so the containment test is done in
    the master's own frame and never in screen pixels.
    """
    points = [(float(entry["x"]), float(entry["y"])) for entry in polygon]
    if len(points) < 3:
        raise WorkbenchError("a lasso selection needs at least three points")
    xs = [x for x, _ in points]
    ys = [y for _, y in points]
    cells = []
    for y in range(max(0, int(min(ys))), min(member.height, int(max(ys)) + 1)):
        for x in range(max(0, int(min(xs))), min(member.width, int(max(xs)) + 1)):
            if _inside(points, x + 0.5, y + 0.5):
                cells.append((x, y))
    return cells


def _inside(points, x: float, y: float) -> bool:
    inside = False
    count = len(points)
    for index in range(count):
        x0, y0 = points[index]
        x1, y1 = points[(index + 1) % count]
        if (y0 > y) != (y1 > y):
            crossing = x0 + (y - y0) * (x1 - x0) / (y1 - y0)
            if x < crossing:
                inside = not inside
    return inside


def bounds(cells) -> dict:
    xs = [x for x, _ in cells]
    ys = [y for _, y in cells]
    return {
        "min_x": min(xs),
        "min_y": min(ys),
        "max_x": max(xs),
        "max_y": max(ys),
        "width": max(xs) - min(xs) + 1,
        "height": max(ys) - min(ys) + 1,
    }


def mask_bytes(member: Member, cells) -> bytes:
    """The exact covered set, as a portable bitmap over the selection's box.

    PBM because it needs no library to write, no library to read, and diffs as
    text. The mask is referenced from the packet by path and digest, so a mask
    edited after the fact stops matching the address it belongs to.
    """
    box = bounds(cells)
    covered = set(cells)
    rows = [
        "".join(
            "1" if (box["min_x"] + dx, box["min_y"] + dy) in covered else "0"
            for dx in range(box["width"])
        )
        for dy in range(box["height"])
    ]
    header = (
        "P1\n"
        f"# workbench selection mask, member {member.member}\n"
        f"# origin {box['min_x']},{box['min_y']} in the authored cell lattice\n"
        f"{box['width']} {box['height']}\n"
    )
    return (header + "\n".join(rows) + "\n").encode("utf-8")


def build(
    *,
    projection: Projection,
    member: Member,
    gesture: str,
    cells,
    screen_region: dict | None,
    comment: str,
    selection_id: str,
    created_at: str,
    repository_revision: str | None,
    mask_reference: dict | None,
    geometry: dict | None = None,
    capture: dict | None = None,
    author: str = "owner",
) -> dict:
    """Assemble one packet. Every field is either measured or bound; none is guessed.

    `capture` is a capture's contribution as plain data — its bound digests, its
    camera, its frame generation, and what the frame showed under the gesture —
    or None for a logical selection. **One packet type covers both views**: the
    same fields are present either way, and which view produced it is a value
    rather than a shape.

    `geometry` is the gesture exactly as it was made, in the space the view uses.
    It is recorded because it is what a consumer re-derives the address from; a
    packet whose cells could not be re-derived would be a claim rather than a
    measurement.
    """
    resolution = resolve(member, cells)
    box = bounds(cells)
    tile = projection.tile_size_px
    return {
        "schema_version": SCHEMA_VERSION,
        "kind": PACKET_KIND,
        "workbench_version": VERSION,
        "selection_id": selection_id,
        "created_at": created_at,
        # Who pointed. An owner gesture and an agent proposal are the same
        # record differing only here, which is the agent-parity law in one field.
        "author": author,
        "source": {
            # The revision orients a reader; it never decides staleness. A tree
            # is routinely dirty while the owner works, and refusing every
            # selection taken on uncommitted work would be theatre. The digests
            # below are the binding.
            "repository_revision": repository_revision,
            "revision_binding": "advisory",
            "digest_binding": "fail_closed",
            "digests": (
                projection.source_records()
                if capture is None
                else projection.source_records() + list(capture["sources"])
            ),
        },
        "view": VIEW_LOGICAL if capture is None else VIEW_CAPTURE,
        "scene": {
            "land_id": projection.land_id,
            "realm_id": projection.realm_id,
            "member": member.member,
            # A logical selection is taken against a compiled document, not a
            # rendered frame, so it binds no frame generation. A capture binds
            # the generation of the frame it photographed.
            "frame_generation": None if capture is None else capture["frame_generation"],
        },
        # The logical view has no camera. The field stays, and stays null, so a
        # capture packet and a logical packet remain the same record type.
        "camera": None if capture is None else capture["camera"],
        "screen_region": {
            "gesture": gesture,
            # The space the geometry below is expressed in, and therefore the
            # space `canvas_rect` is in too.
            "space": SPACE_CELLS if capture is None else SPACE_PIXELS,
            # The gesture as it was made. Authoritative for a capture selection,
            # because the cells are derived from it through the identity raster;
            # recorded for a logical one for the same reason, so both views are
            # equally re-derivable by a consumer that trusts nothing.
            "geometry": geometry,
            "canvas_rect": screen_region,
            "cell_bounds": box,
            "mask": mask_reference,
        },
        "cells": resolution["cells"],
        "world": {
            "frame": "authored_cell_lattice",
            "member": member.member,
            "tile_size_px": tile,
            "cell_bounds": box,
            "pixel_bounds": {
                "x": box["min_x"] * tile,
                "y": box["min_y"] * tile,
                "width": box["width"] * tile,
                "height": box["height"] * tile,
            },
            # Elevation is a member-level fact in this land: the surface and the
            # interior are separate members, not bands within one.
            "layer_band": None,
        },
        "semantic": resolution["semantic"],
        "candidates": resolution["candidates"],
        "ambiguous": resolution["ambiguous"],
        # What the captured frame actually showed under the gesture: actors,
        # corpses, loose items, gold. Runtime identities, kept out of `semantic`
        # deliberately — see the module note below — and empty for a logical
        # selection, which photographs nothing.
        "observed": [] if capture is None else list(capture["observed"]),
        # A capture IS the context image: the picture the owner pointed at, bound
        # by digest like every other source.
        "context_image": None if capture is None else capture["context_image"],
        # V0 has no image operations, so nothing may be replaced and no commit
        # mask can honestly be named — capture view or not.
        "commit_mask": None,
        "comment": comment,
    }


def resolution_of(projection: Projection, packet: dict) -> dict:
    """Re-resolve a packet's cells through the one resolver.

    This is what makes agent parity mechanical: the server and `resolve.py`
    both answer with this, so "the same identities" is one code path rather
    than two implementations that agree today.
    """
    member = projection.member(packet["scene"]["member"])
    cells = [_point(entry) for entry in packet["cells"]]
    outside = [cell for cell in cells if not member.contains(cell)]
    if outside:
        raise WorkbenchError(
            f"packet {packet.get('selection_id')!r} names cell {outside[0]}, "
            f"which member {member.member!r} does not carry"
        )
    return resolve(member, cells)
