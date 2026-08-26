#!/usr/bin/env python3
"""Resolve a selection packet the way a consumer must: fail-closed, then exact.

This is the reference consumer. An agent needs nothing else — no browser, no
server, no build — to turn a packet into the identities the owner pointed at:

    python3 tools/workbench/resolve.py .workbench/sessions/<id>/selections/sel-0001.json

It refuses before it answers. Every bound digest is recomputed, the mask is
checked against the cells it claims to encode, the gesture is re-derived from
the geometry the packet recorded, and the packet's identities are re-derived
from the current projection and compared. Any disagreement is a refusal naming
what moved, never a best-effort answer.

A **capture** packet gets every one of those checks and three more, because it
binds three more files. The picture, the identity sidecar, and the identity
raster must still describe each other; the sidecar's frame, camera, and viewport
must be the ones the packet recorded; and the gesture is replayed through the
raster so that the cells and the observed identities are re-derived from the
capture rather than trusted from the packet. A capture packet whose sidecar was
edited resolves to nothing at all.

Exit codes: 0 resolved, 2 refused, 3 the packet or the tree could not be read.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):  # invoked as a script, the ordinary agent path
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from workbench import capture as capture_reader  # noqa: E402
from workbench.imageops.masks import MaskUnreadable, decode_p1  # noqa: E402
from workbench.packet import (  # noqa: E402
    PACKET_KIND,
    VIEW_CAPTURE,
    VIEW_LOGICAL,
    cells_for_gesture,
    resolution_of,
)
from workbench.packet import SCHEMA_VERSION as PACKET_SCHEMA  # noqa: E402
from workbench.projection import (  # noqa: E402
    PROJECTION_ROLE,
    ProjectionUnavailable,
    Source,
    StaleSelection,
    WorkbenchError,
    digest_file,
    load,
    verify,
)

EXIT_OK = 0
EXIT_REFUSED = 2
EXIT_UNREADABLE = 3

#: The structural markers that identify this repository's root. Structural
#: rather than name-based, and the same three the compiler looks for, so the
#: two halves of the bridge can never disagree about which tree they are in.
ROOT_MARKERS = ("Cargo.toml", "content", "tools/run_checks.py")


class Refused(WorkbenchError):
    """The packet cannot be honoured, and the reason names what moved."""


def find_root(start: Path) -> Path:
    for candidate in [start, *start.parents]:
        if all((candidate / marker).exists() for marker in ROOT_MARKERS):
            return candidate
    raise WorkbenchError(f"could not locate the repository root above {start}")


def decode_mask(payload: bytes) -> tuple[int, int, int, int, set[tuple[int, int]]]:
    """Decode a P1 bitmap and the origin its comment records.

    The format is decoded once, in `imageops.masks`, and re-raised here as a
    refusal. A selection mask and a commit mask are the same file format over
    two different address spaces — cells here, source-image pixels there — and
    two decoders of one format are two answers waiting to disagree about a
    malformed file. The **space** stays this module's business; the bytes do
    not.
    """
    try:
        mask = decode_p1(payload)
    except MaskUnreadable as error:
        raise Refused(f"the referenced mask is unreadable: {error}") from error
    return mask.origin_x, mask.origin_y, mask.width, mask.height, set(mask.covered)


def read_packet(path: Path) -> dict:
    try:
        packet = json.loads(path.read_bytes())
    except OSError as error:
        raise WorkbenchError(f"cannot read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise WorkbenchError(f"{path} is not valid JSON: {error}") from error
    if not isinstance(packet, dict) or packet.get("kind") != PACKET_KIND:
        raise WorkbenchError(f"{path} is not a {PACKET_KIND}")
    if packet.get("schema_version") != PACKET_SCHEMA:
        raise WorkbenchError(
            f"{path} declares schema version {packet.get('schema_version')!r}, "
            f"and this consumer reads version {PACKET_SCHEMA}"
        )
    return packet


def check_mask(root: Path, packet: dict) -> None:
    """A mask is part of the address, so it is verified like one."""
    reference = packet["screen_region"].get("mask")
    if not reference:
        return
    path = root / reference["path"]
    actual = digest_file(path)
    if actual is None:
        raise Refused(f"the referenced mask {reference['path']} is missing")
    if actual != reference["sha256"]:
        raise Refused(
            f"the referenced mask {reference['path']} digest moved "
            f"(bound {reference['sha256']}, on disk {actual})"
        )
    _, _, _, _, covered = decode_mask(path.read_bytes())
    cells = {(int(cell["x"]), int(cell["y"])) for cell in packet["cells"]}
    if covered != cells:
        raise Refused(
            f"the referenced mask {reference['path']} encodes {len(covered)} cells "
            f"and the packet names {len(cells)}; they are not the same selection"
        )


def check_capture(root: Path, packet: dict, sources, projection) -> dict:
    """Replay a capture selection through the capture it was taken over.

    The packet's cells are not read here; they are re-derived. A capture binds a
    picture, a sidecar, and a per-pixel identity raster, and the whole point of
    binding them is that the address can be recomputed from them. If the
    recomputation disagrees with what the packet says, the packet is wrong about
    the world and there is nothing to answer.
    """
    sidecar_paths = [
        source.path for source in sources if source.role == capture_reader.SIDECAR_ROLE
    ]
    if len(sidecar_paths) != 1:
        raise Refused("the packet does not bind exactly one capture identity sidecar")
    directory = (root / sidecar_paths[0]).parent
    taken = capture_reader.load(root, directory)

    member = capture_reader.bind(projection, taken)
    if member != packet["scene"]["member"]:
        raise Refused(
            f"the capture was taken on {member!r} and the packet names member "
            f"{packet['scene']['member']!r}"
        )
    if taken.frame_generation != packet["scene"]["frame_generation"]:
        raise Refused(
            f"the capture photographed frame generation {taken.frame_generation} "
            f"and the packet names {packet['scene']['frame_generation']}"
        )
    recorded_camera = packet["camera"] or {}
    if recorded_camera != {**taken.camera, "viewport": taken.viewport}:
        raise Refused("the packet's camera identity is not the one the sidecar records")

    region = packet["screen_region"]
    if region.get("space") != "capture_image_pixels":
        raise Refused("a capture selection must record its gesture in capture image pixels")
    geometry = region.get("geometry")
    if not isinstance(geometry, dict):
        raise Refused("the capture selection records no gesture geometry to re-derive")
    indices = capture_reader.indices_for_gesture(taken.raster, region["gesture"], geometry)
    fresh_cells = [
        {"x": x, "y": y} for x, y in capture_reader.cells_for_indices(taken, indices)
    ]
    if fresh_cells != packet["cells"]:
        raise Refused(
            "replaying the gesture through the identity raster names different cells "
            "than the packet does; the packet has been edited since it was written"
        )
    fresh_observed = capture_reader.observed(taken, indices)
    if json.loads(json.dumps(fresh_observed)) != json.loads(json.dumps(packet["observed"])):
        raise Refused(
            "replaying the gesture names different observed identities than the packet does"
        )
    return {
        "directory": taken.relative,
        "frame_generation": taken.frame_generation,
        "viewport": taken.viewport,
        "camera": taken.camera,
        "targets_in_frame": len(taken.targets),
        "observed": fresh_observed,
    }


def check_geometry(projection, packet: dict) -> None:
    """Re-derive a logical selection's cells from the gesture it recorded."""
    region = packet["screen_region"]
    geometry = region.get("geometry")
    if not isinstance(geometry, dict):
        raise Refused("the selection records no gesture geometry to re-derive")
    if region.get("space") != "authored_cell_lattice":
        raise Refused("a logical selection must record its gesture in the authored cell lattice")
    member = projection.member(packet["scene"]["member"])
    fresh = [
        {"x": x, "y": y} for x, y in cells_for_gesture(member, region["gesture"], geometry)
    ]
    if fresh != packet["cells"]:
        raise Refused(
            "re-deriving the gesture names different cells than the packet does; "
            "the packet has been edited since it was written"
        )


def resolve_packet(path: Path, root: Path | None = None) -> dict:
    """Verify a packet against the tree, then resolve it. Refuses or answers."""
    path = Path(path).resolve()
    packet = read_packet(path)
    root = Path(root).resolve() if root else find_root(path.parent)

    sources = [Source.from_record(record) for record in packet["source"]["digests"]]
    verify(root, sources)
    check_mask(root, packet)

    projection_paths = [source.path for source in sources if source.role == PROJECTION_ROLE]
    if len(projection_paths) != 1:
        raise Refused("the packet does not bind exactly one logical projection")
    projection = load(root, projection_paths[0])

    view = packet["view"]
    if view not in (VIEW_LOGICAL, VIEW_CAPTURE):
        raise Refused(f"the packet declares an unknown view {view!r}")
    capture_record = None
    if view == VIEW_CAPTURE:
        capture_record = check_capture(root, packet, sources, projection)
    else:
        check_geometry(projection, packet)

    resolution = resolution_of(projection, packet)
    recorded = {
        "semantic": packet["semantic"],
        "candidates": packet["candidates"],
        "ambiguous": packet["ambiguous"],
    }
    fresh = {
        "semantic": resolution["semantic"],
        "candidates": resolution["candidates"],
        "ambiguous": resolution["ambiguous"],
    }
    if json.loads(json.dumps(recorded)) != json.loads(json.dumps(fresh)):
        raise Refused(
            "the packet's recorded identities disagree with the current projection; "
            "the packet has been edited since it was written"
        )
    return {
        "packet_path": str(path),
        "repository_root": str(root),
        "selection_id": packet["selection_id"],
        "verified_digests": [source.as_record() for source in sources],
        "view": packet["view"],
        "scene": packet["scene"],
        "gesture": packet["screen_region"]["gesture"],
        "mask": packet["screen_region"].get("mask"),
        "capture": capture_record,
        "resolution": resolution,
        "observed": packet["observed"],
        "comment": packet["comment"],
    }


def render(answer: dict) -> str:
    lines = [
        f"packet      {answer['packet_path']}",
        f"selection   {answer['selection_id']}  view={answer['view']}  "
        f"member={answer['scene']['member']}  gesture={answer['gesture']}",
        f"binding     VERIFIED, {len(answer['verified_digests'])} digests:",
    ]
    lines.extend(
        f"              {record['role']:<20} {record['sha256'][:12]}  {record['path']}"
        for record in answer["verified_digests"]
    )
    if answer["mask"]:
        lines.append(f"mask        {answer['mask']['path']}  {answer['mask']['sha256'][:12]}")
    if answer["capture"]:
        taken = answer["capture"]
        lines.append(
            f"capture     {taken['directory']}  frame generation {taken['frame_generation']}"
            f"  {taken['viewport']['width']}x{taken['viewport']['height']}"
            f"  {taken['targets_in_frame']} targets in frame"
        )
    resolution = answer["resolution"]
    cells = resolution["cells"]
    shown = " ".join(f"{cell['x']},{cell['y']}" for cell in cells[:16])
    more = "" if len(cells) <= 16 else f" ... (+{len(cells) - 16} more)"
    lines.append(f"cells       {len(cells)}: {shown}{more}")
    lines.append(
        "ambiguous   "
        + ("YES — a consumer must ask, not pick" if resolution["ambiguous"] else "no")
    )
    lines.append("identities")
    for record in resolution["semantic"]:
        coverage = record["coverage"]
        lines.append(
            f"  {record['rank']:>2}. {record['kind']:<12} {record['identity']}"
            f"   selection={coverage['selection_coverage']:.2f}"
            f" identity={coverage['identity_coverage']:.2f}"
        )
        detail = " ".join(
            f"{key}={value}"
            for key, value in record["detail"].items()
            if not isinstance(value, (dict, list))
        )
        if detail:
            lines.append(f"      {detail}")
    if answer["observed"]:
        lines.append("observed    (what the frame showed; runtime, not addressable land)")
        for record in answer["observed"]:
            lines.append(
                f"  {record['kind']:<12} {record['identity']}"
                f"   at {record['coordinate']['x']},{record['coordinate']['y']}"
            )
    lines.append("comment")
    lines.append(f"  {answer['comment']!r}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("packet", help="path to a selection packet")
    parser.add_argument("--root", default=None, help="repository root (default: inferred)")
    parser.add_argument(
        "--json", action="store_true", help="emit the resolved answer as JSON"
    )
    arguments = parser.parse_args(argv)
    try:
        answer = resolve_packet(Path(arguments.packet), arguments.root)
    except (StaleSelection, Refused) as refusal:
        print(f"REFUSED: {refusal}", file=sys.stderr)
        return EXIT_REFUSED
    except (ProjectionUnavailable, WorkbenchError) as error:
        print(f"UNREADABLE: {error}", file=sys.stderr)
        return EXIT_UNREADABLE
    if arguments.json:
        print(json.dumps(answer, indent=2))
    else:
        print(render(answer))
    return EXIT_OK


if __name__ == "__main__":
    sys.exit(main())
