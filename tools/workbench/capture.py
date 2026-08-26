"""The gameplay capture: a real client frame, and the identities in its pixels.

A capture is three files written together by the client's own presenter — the
picture, an identity sidecar, and a per-pixel identity raster — and they are
only meaningful as a set. This module requests one, reads all three, and turns
a gesture over the picture into the same cells and the same identities a
gesture over the logical view would produce.

**Nothing here runs anything.** Reading a capture and selecting over one are
file reads and arithmetic. Taking a new capture is a real client run, and it
lives alone in `capture_harness.py` so that the boundary between "the ordinary
selection path" and "the expensive thing" is a module boundary a test can
check, not a convention. `tests/test_workbench_loop.py` holds that line.

**Why the raster decides.** The sidecar's target list carries a rectangle per
target, so a consumer could intersect rectangles instead. The raster is used
because it is what the presenter actually drew: markers overwrite the squares
beneath them, and the raster records the result of that overwriting rather than
a rule for reproducing it. One authority, no tie-breaks, and lasso and paint
over a capture are exact per pixel rather than nearest-anchor guesses.
"""

from __future__ import annotations

import json
import struct
from dataclasses import dataclass
from pathlib import Path

from .imageops.png import ImageUnreadable
from .imageops.png import size as png_dimensions
from .projection import Projection, WorkbenchError, digest_bytes

#: Where a session keeps its captures. One directory per capture, holding the
#: three files exactly as the client wrote them.
CAPTURES_DIR = "captures"

SIDECAR_NAME = "capture.sidecar.json"
IMAGE_NAME = "capture.png"
RASTER_NAME = "capture.identity.pgm"

SIDECAR_KIND = "capture_identity_sidecar"
SIDECAR_SCHEMA_VERSION = 1
RASTER_FORMAT = "pgm_p5_u16_be_target_index"

#: Roles the capture adds to a packet's bound digest set. Each is an
#: independent mutant: moving any one of them kills the packet on its own.
IMAGE_ROLE = "capture_image"
SIDECAR_ROLE = "capture_sidecar"
RASTER_ROLE = "capture_identity_raster"


class CaptureUnavailable(WorkbenchError):
    """A capture cannot be taken, and the reason names what is missing.

    Honest unavailability rather than a placeholder picture: a capture whose
    pixels show nothing would still carry a sidecar claiming they showed
    something, and that is a confident wrong answer.
    """


def png_size(payload: bytes) -> tuple[int, int]:
    """The width and height the capture image declares, for the sidecar check.

    This module used to parse the header itself, in ten lines, because the
    Workbench takes no dependency outside the standard library and this was the
    only thing it needed to know about the picture. `imageops.png` is now that
    reader — still standard library, still no dependency — and there is one PNG
    parser in the tree rather than two that could disagree about what a header
    says. What is checked here is unchanged: whether the sidecar's viewport size
    is the truth.
    """
    try:
        return png_dimensions(payload)
    except ImageUnreadable as error:
        raise WorkbenchError(f"the capture image is not a PNG: {error}") from error


@dataclass(frozen=True)
class Raster:
    """The identity raster: one 16-bit target index per pixel, zero for none."""

    width: int
    height: int
    samples: bytes

    def index_at(self, x: int, y: int) -> int:
        if x < 0 or y < 0 or x >= self.width or y >= self.height:
            return 0
        offset = (y * self.width + x) * 2
        return (self.samples[offset] << 8) | self.samples[offset + 1]

    def indices_in_row(self, y: int, x0: int, x1: int) -> set[int]:
        """Every index recorded in one row between two inclusive columns."""
        if y < 0 or y >= self.height:
            return set()
        x0 = max(0, x0)
        x1 = min(self.width - 1, x1)
        if x1 < x0:
            return set()
        start = (y * self.width + x0) * 2
        stop = (y * self.width + x1 + 1) * 2
        span = self.samples[start:stop]
        return {value for value in struct.unpack(f">{(stop - start) // 2}H", span) if value}


def read_raster(payload: bytes) -> Raster:
    """Decode a binary Netpbm greyscale, or refuse naming what is wrong."""
    if not payload.startswith(b"P5"):
        raise WorkbenchError("the identity raster is not a binary Netpbm greyscale")
    fields: list[int] = []
    offset = 2
    while len(fields) < 3:
        while offset < len(payload) and payload[offset : offset + 1].isspace():
            offset += 1
        if payload[offset : offset + 1] == b"#":
            while offset < len(payload) and payload[offset : offset + 1] != b"\n":
                offset += 1
            continue
        start = offset
        while offset < len(payload) and not payload[offset : offset + 1].isspace():
            offset += 1
        if start == offset:
            raise WorkbenchError("the identity raster header ended early")
        fields.append(int(payload[start:offset]))
    offset += 1  # exactly one whitespace byte separates the header from the samples
    width, height, maximum = fields
    if maximum != 65535:
        raise WorkbenchError(
            f"the identity raster declares a maximum of {maximum}; a target index needs 65535"
        )
    samples = payload[offset:]
    if len(samples) != width * height * 2:
        raise WorkbenchError(
            f"the identity raster declares {width}x{height} and carries {len(samples)} bytes"
        )
    return Raster(width=width, height=height, samples=samples)


@dataclass(frozen=True)
class Capture:
    """One capture, its three files, and their digests."""

    directory: Path
    relative: str
    document: dict
    image: bytes
    raster: Raster
    image_digest: str
    sidecar_digest: str
    raster_digest: str

    @property
    def targets(self) -> list[dict]:
        return list(self.document["targets"])

    def target(self, index: int) -> dict:
        return self.document["targets"][index - 1]

    @property
    def camera(self) -> dict:
        return dict(self.document["camera"])

    @property
    def viewport(self) -> dict:
        return dict(self.document["viewport"])

    @property
    def frame_generation(self) -> int:
        return int(self.document["frame_generation"])

    @property
    def level(self) -> str:
        return str(self.document["scene"]["level"])

    @property
    def realm(self) -> str:
        return str(self.document["scene"]["realm"])

    def relative_path(self, root: Path, name: str) -> str:
        return str(self.directory.relative_to(root) / name)

    def binding(self, root: Path, observed_records: list[dict]) -> dict:
        """Everything a capture contributes to a selection packet.

        Handed to the packet builder as plain data so that the builder — which
        is on the ordinary selection path — never imports the capture machinery
        at all.
        """
        return {
            "sources": self.source_records(root),
            # Pose, projection, viewport, and every framing constant in force.
            # There is exactly one framing rule here and the camera states it.
            "camera": {**self.camera, "viewport": self.viewport},
            "frame_generation": self.frame_generation,
            "scene": {"realm": self.realm, "level": self.level},
            "context_image": {
                "path": self.relative_path(root, IMAGE_NAME),
                "sha256": self.image_digest,
            },
            "observed": observed_records,
        }

    def source_records(self, root: Path) -> list[dict[str, str]]:
        base = self.directory.relative_to(root)
        return [
            {"role": IMAGE_ROLE, "path": str(base / IMAGE_NAME), "sha256": self.image_digest},
            {"role": SIDECAR_ROLE, "path": str(base / SIDECAR_NAME), "sha256": self.sidecar_digest},
            {"role": RASTER_ROLE, "path": str(base / RASTER_NAME), "sha256": self.raster_digest},
        ]


def load(root: Path, directory: Path) -> Capture:
    """Read a capture directory and check that its three files describe each other.

    The sidecar names the digests of the picture and the raster it was written
    beside. Those are recomputed here, before anything is resolved, so a capture
    whose picture was replaced cannot be selected over at all.
    """
    directory = Path(directory)
    try:
        payload = (directory / SIDECAR_NAME).read_bytes()
    except OSError as error:
        raise CaptureUnavailable(f"no capture sidecar at {directory}: {error}") from error
    try:
        document = json.loads(payload)
    except json.JSONDecodeError as error:
        raise CaptureUnavailable(f"{directory}/{SIDECAR_NAME} is not valid JSON: {error}") from error
    if document.get("kind") != SIDECAR_KIND:
        raise CaptureUnavailable(
            f"{directory}/{SIDECAR_NAME} declares kind {document.get('kind')!r}, not {SIDECAR_KIND!r}"
        )
    if document.get("schema_version") != SIDECAR_SCHEMA_VERSION:
        raise CaptureUnavailable(
            f"{directory}/{SIDECAR_NAME} declares schema version "
            f"{document.get('schema_version')!r}, and this Workbench reads "
            f"version {SIDECAR_SCHEMA_VERSION}"
        )
    if document.get("identity_raster", {}).get("format") != RASTER_FORMAT:
        raise CaptureUnavailable(
            f"the identity raster format is {document.get('identity_raster', {}).get('format')!r}, "
            f"and this Workbench reads {RASTER_FORMAT!r}"
        )

    try:
        image = (directory / IMAGE_NAME).read_bytes()
        raster_payload = (directory / RASTER_NAME).read_bytes()
    except OSError as error:
        raise CaptureUnavailable(f"the capture at {directory} is incomplete: {error}") from error

    image_digest = digest_bytes(image)
    raster_digest = digest_bytes(raster_payload)
    if image_digest != document["image"]["sha256"]:
        raise CaptureUnavailable(
            f"{directory}/{IMAGE_NAME} does not hold the bytes its sidecar names"
        )
    if raster_digest != document["identity_raster"]["sha256"]:
        raise CaptureUnavailable(
            f"{directory}/{RASTER_NAME} does not hold the bytes its sidecar names"
        )

    width, height = png_size(image)
    if (width, height) != (int(document["viewport"]["width"]), int(document["viewport"]["height"])):
        raise CaptureUnavailable(
            f"the sidecar claims a {document['viewport']['width']}x{document['viewport']['height']} "
            f"viewport and the picture is {width}x{height}"
        )
    raster = read_raster(raster_payload)
    if (raster.width, raster.height) != (width, height):
        raise CaptureUnavailable(
            f"the identity raster is {raster.width}x{raster.height} and the picture is {width}x{height}"
        )
    for position, record in enumerate(document["targets"], start=1):
        if int(record["index"]) != position:
            raise CaptureUnavailable(
                "the sidecar's target list is not indexed from one in order"
            )

    return Capture(
        directory=directory,
        relative=str(directory.relative_to(root)),
        document=document,
        image=image,
        raster=raster,
        image_digest=image_digest,
        sidecar_digest=digest_bytes(payload),
        raster_digest=raster_digest,
    )


def bind(projection: Projection, capture: Capture) -> str:
    """The projection member this capture addresses, or a refusal.

    A capture of another realm, or of a level the compiled land does not carry,
    is not a selection surface for this projection. Saying so here is what stops
    a capture selection from resolving into the wrong land's cells.
    """
    if capture.realm != projection.realm_id:
        raise CaptureUnavailable(
            f"the capture was taken in realm {capture.realm!r} and this projection "
            f"compiles realm {projection.realm_id!r}"
        )
    if capture.level not in projection.members:
        raise CaptureUnavailable(
            f"the capture was taken on level {capture.level!r} and this projection "
            f"carries members {sorted(projection.members)}"
        )
    return capture.level


# -- gestures over a capture -------------------------------------------------


def indices_for_gesture(raster: Raster, gesture: str, geometry: dict) -> set[int]:
    """Every target index the gesture's pixels cover.

    Coverage is per pixel and is not weighted: one pixel of a square is that
    square. The logical view behaves the same way at cell granularity, which is
    what keeps the two views' answers comparable.
    """
    if gesture == "click":
        point = geometry["point"]
        index = raster.index_at(int(point["x"]), int(point["y"]))
        return {index} if index else set()
    if gesture == "box":
        rect = geometry["rect"]
        x0, y0 = int(rect["x"]), int(rect["y"])
        width, height = int(rect["width"]), int(rect["height"])
        if width <= 0 or height <= 0:
            raise WorkbenchError("a box selection must cover at least one pixel")
        covered: set[int] = set()
        for y in range(y0, y0 + height):
            covered |= raster.indices_in_row(y, x0, x0 + width - 1)
        return covered
    if gesture == "lasso":
        return _lasso_indices(raster, geometry["polygon"])
    if gesture == "paint":
        covered = set()
        for point in geometry["points"]:
            index = raster.index_at(int(point["x"]), int(point["y"]))
            if index:
                covered.add(index)
        return covered
    raise WorkbenchError(f"unknown gesture {gesture!r} over a capture")


def _lasso_indices(raster: Raster, polygon) -> set[int]:
    """Scanline fill in image pixels, by the even-odd rule.

    Rows are resolved into spans and the spans are read from the raster in one
    slice each, so a lasso over a large region costs a handful of slices per row
    rather than a lookup per pixel.
    """
    points = [(float(entry["x"]), float(entry["y"])) for entry in polygon]
    if len(points) < 3:
        raise WorkbenchError("a lasso selection needs at least three points")
    top = max(0, int(min(y for _, y in points)))
    bottom = min(raster.height - 1, int(max(y for _, y in points)))
    covered: set[int] = set()
    for y in range(top, bottom + 1):
        centre = y + 0.5
        crossings = []
        for index in range(len(points)):
            x0, y0 = points[index]
            x1, y1 = points[(index + 1) % len(points)]
            if (y0 > centre) != (y1 > centre):
                crossings.append(x0 + (centre - y0) * (x1 - x0) / (y1 - y0))
        crossings.sort()
        for pair in range(0, len(crossings) - 1, 2):
            covered |= raster.indices_in_row(
                y, int(crossings[pair] + 0.5), int(crossings[pair + 1] - 0.5)
            )
    return covered


def observed(capture: Capture, indices) -> list[dict]:
    """What the frame showed at the covered pixels, in target-list order.

    Every target the gesture touched, squares included, exactly as the presenter
    recorded it: identity, kind, square, presentation layer, anchor, and hit
    shape. Two things depend on it being the whole covered set rather than a
    selection of interesting parts — a consumer re-derives the packet's cells
    from it, and the browser highlights what was covered from it.

    It is deliberately **not** merged into the packet's semantic set. The
    semantic set names the authored world an agent can act on. What the frame
    happened to be showing at the instant of the photograph is a fact about that
    instant; folding it in would break the equality with a logical selection and
    invite a consumer to address a transient as though it were a place.
    """
    records = []
    for index in sorted(indices):
        record = capture.target(index)
        records.append({
            "index": index,
            "identity": record["identity"],
            "kind": record["kind"],
            "source_identity": record["source_identity"],
            "coordinate": dict(record["coordinate"]),
            "presentation_layer": record["presentation_layer"],
            "anchor": dict(record["anchor"]),
            "hit_shape": dict(record["hit_shape"]),
        })
    return records


def cells_for_indices(capture: Capture, indices) -> list[tuple[int, int]]:
    """The squares the covered targets stand on, row-major and deduplicated."""
    squares = set()
    for index in indices:
        record = capture.target(index)
        squares.add((int(record["coordinate"]["x"]), int(record["coordinate"]["y"])))
    return sorted(squares, key=lambda cell: (cell[1], cell[0]))


def select(projection: Projection, capture: Capture, gesture: str, geometry: dict) -> dict:
    """One gesture over one capture, resolved into the land's own address space.

    The whole capture path is here rather than in the server, so that the
    browser, an agent, and a test all reach the same six lines. A gesture is
    pixels; a pixel is a target; a target stands on a square; a square is a cell
    of the compiled member. Nothing along that chain is estimated.
    """
    member_name = bind(projection, capture)
    member = projection.member(member_name)
    indices = indices_for_gesture(capture.raster, gesture, geometry)
    if not indices:
        raise WorkbenchError("the gesture covered no addressable pixel of the capture")
    cells = cells_for_indices(capture, indices)
    outside = [cell for cell in cells if not member.contains(cell)]
    if outside:
        raise WorkbenchError(
            f"the capture names cell {outside[0]}, which member {member.member!r} does not "
            "carry; the capture and the projection are not the same land"
        )
    return {
        "member": member,
        "gesture": gesture,
        "geometry": geometry,
        "indices": indices,
        "cells": cells,
        "observed": observed(capture, indices),
    }


def canvas_rect(gesture: str, geometry: dict) -> dict:
    """The gesture's bounding box in image pixels, for a human reading the packet."""
    if gesture == "click":
        point = geometry["point"]
        return {"x": int(point["x"]), "y": int(point["y"]), "width": 1, "height": 1}
    if gesture == "box":
        rect = geometry["rect"]
        return {
            "x": int(rect["x"]),
            "y": int(rect["y"]),
            "width": int(rect["width"]),
            "height": int(rect["height"]),
        }
    points = geometry["polygon"] if gesture == "lasso" else geometry["points"]
    xs = [float(point["x"]) for point in points]
    ys = [float(point["y"]) for point in points]
    return {
        "x": int(min(xs)),
        "y": int(min(ys)),
        "width": max(1, int(max(xs)) - int(min(xs)) + 1),
        "height": max(1, int(max(ys)) - int(min(ys)) + 1),
    }


def next_capture_id(session_directory: Path) -> str:
    """The next capture identifier in a session, in the order they were taken."""
    directory = Path(session_directory) / CAPTURES_DIR
    existing = sorted(path.name for path in directory.glob("cap-*")) if directory.is_dir() else []
    return f"cap-{len(existing) + 1:04d}"
