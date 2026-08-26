#!/usr/bin/env python3
"""Rebuild the tracked synthetic Workbench session fixture.

    python3 tests/fixtures/workbench/regenerate.py

**Why this fixture exists.** The Workbench's working state lives under an
ignored root, and tracked proof may never depend on an ignored root. A clean
clone therefore carries this: a complete, self-contained, entirely synthetic
land plus one session over it, with real files and real digests, so the session
shape, the agent read path, and the fail-closed refusals are all provable
without anyone ever having opened the browser application.

**Why it is generated rather than hand-written.** Every packet in it binds
digests of the files beside it. Hand-maintaining those is how a fixture quietly
stops meaning anything. The packet is built by the same builder the running
Workbench uses, so the fixture cannot drift from the shape the tool emits, and
`tests/test_workbench_fixture.py` asserts the tracked bytes are exactly what
this script writes.

**What it is not.** The land here is invented and carries no content authority
whatsoever. It is not the authoring fixture, it is not geography, and nothing
outside these tests may read it.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parents[2] / "tools"))

from workbench import VERSION  # noqa: E402
from workbench.packet import build, cells_for_gesture, mask_bytes  # noqa: E402
from workbench.projection import digest_bytes, load  # noqa: E402
from workbench.session import (  # noqa: E402
    COMMENT_RECORD,
    MANIFEST_KIND,
    SELECTION_RECORD,
    RETENTION_STATEMENT,
)

# Fixed so a regeneration is byte-identical. A fixture with a wall clock in it
# is a fixture that fails on Tuesdays.
STAMP = "2026-08-19T00:00:00Z"
SESSION_ID = "session-fixture"
REVISION = "0000000000000000000000000000000000000000"

PROJECTION_PATH = "projection/synthetic-logical-projection.json"
SOURCES = {
    "master": "sources/synthetic-master.json",
    "companion": "sources/synthetic-companion.json",
    "receipt": "sources/synthetic-receipt.json",
    "runtime_projection": "sources/synthetic-runtime-projection.json",
}

GROUND = "synthetic_ground"
WATER = "synthetic_water"
ROUTE = "synthetic_route"
FOOTPRINT = "synthetic_footprint"
MARK = "synthetic_mark"
FLOOR = "synthetic_floor"
WALL = "synthetic_wall"


def cell(x: int, y: int, terrain: list[tuple[str, str]], passable: bool) -> dict:
    return {
        "x": x,
        "y": y,
        "passable": passable,
        "terrain": [{"class": name, "layer": layer} for name, layer in terrain],
    }


def surface_member() -> dict:
    """A 4x3 land shaped to exercise every identity kind and real ambiguity.

        row 0   ground  ground  ground+mark   ground
        row 1   ground+route  ground+route  ground+footprint  ground+footprint
        row 2   water   water   ground        ground

    The landmark stands on a route cell, so pointing at it is genuinely
    ambiguous; the structure spans two cells with its access cell elsewhere, so
    partial coverage is exercised too.
    """
    ground = [(GROUND, "base_terrain")]
    cells = [
        cell(0, 0, ground, True),
        cell(1, 0, ground, True),
        cell(2, 0, ground + [(MARK, "landmark_marks")], True),
        cell(3, 0, ground, True),
        cell(0, 1, ground + [(ROUTE, "routes")], True),
        cell(1, 1, ground + [(ROUTE, "routes")], True),
        cell(2, 1, ground + [(FOOTPRINT, "structure_footprints")], False),
        cell(3, 1, ground + [(FOOTPRINT, "structure_footprints")], False),
        cell(0, 2, [(WATER, "base_terrain")], False),
        cell(1, 2, [(WATER, "base_terrain")], False),
        cell(2, 2, ground, True),
        cell(3, 2, ground, True),
    ]
    return {
        "member": "synthetic_surface",
        "width": 4,
        "height": 3,
        "cells": cells,
        "routes": [{"x": 0, "y": 1}, {"x": 1, "y": 1}],
        "structures": [
            {
                "id": "synthetic_hall",
                "purpose": "synthetic_purpose",
                "scope": "clustered",
                "x": 2,
                "y": 1,
                "width": 2,
                "height": 1,
                "access": {"x": 2, "y": 2},
                "facade_door": {"x": 2, "y": 1},
            }
        ],
        "landmarks": [
            {"id": "synthetic_landing", "role": "arrival", "at": {"x": 1, "y": 1}}
        ],
        "transitions": [
            {
                "id": "synthetic_descent",
                "member": "synthetic_surface",
                "target_member": "synthetic_interior",
                "paired_transition": "synthetic_ascent",
                "direction": "down",
                "marker": {"x": 2, "y": 0},
                "access": {"x": 3, "y": 0},
            }
        ],
    }


def interior_member() -> dict:
    """A 2x2 companion, so the transition the surface declares actually lands."""
    floor = [(FLOOR, "base_terrain")]
    return {
        "member": "synthetic_interior",
        "width": 2,
        "height": 2,
        "cells": [
            cell(0, 0, [(WALL, "base_terrain")], False),
            cell(1, 0, floor, True),
            cell(0, 1, floor, True),
            cell(1, 1, floor, True),
        ],
        "routes": [],
        "structures": [],
        "landmarks": [],
        "transitions": [
            {
                "id": "synthetic_ascent",
                "member": "synthetic_interior",
                "target_member": "synthetic_surface",
                "paired_transition": "synthetic_descent",
                "direction": "up",
                "marker": {"x": 1, "y": 0},
                "access": {"x": 1, "y": 1},
            }
        ],
    }


def write(path: Path, payload: bytes) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(payload)
    return digest_bytes(payload)


def json_bytes(value) -> bytes:
    return (json.dumps(value, indent=2) + "\n").encode("utf-8")


def build_sources(out: Path) -> dict[str, str]:
    """Write the four addressed source files and return their digests.

    Their contents are deliberately trivial. What is under test is the binding —
    that a consumer refuses when any one of these files moves — and a digest
    does not care what it is a digest of.
    """
    digests = {}
    for role, relative in SOURCES.items():
        digests[role] = write(
            out / relative,
            json_bytes(
                {
                    "synthetic": True,
                    "role": role,
                    "note": "A stand-in for an addressed source file. No authority, no meaning.",
                }
            ),
        )
    return digests


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    # The fixture-integrity test points this at a temporary directory, so
    # proving the tracked bytes are current never means rewriting them.
    parser.add_argument(
        "--out", default=None, help="write here instead of the tracked fixture"
    )
    arguments = parser.parse_args(argv)
    out = Path(arguments.out).resolve() if arguments.out else HERE
    digests = build_sources(out)
    projection_document = {
        "schema_version": 1,
        "kind": "workbench_logical_projection",
        "land_id": "synthetic_fixture_land",
        "realm_id": "synthetic_realm",
        "candidate_member": "synthetic_surface",
        "tile_size_px": 16,
        "sources": [
            {"role": role, "path": SOURCES[role], "sha256": digests[role]}
            for role in ("master", "companion", "receipt", "runtime_projection")
        ],
        "members": [surface_member(), interior_member()],
        "connectivity": {
            "edges": [
                {
                    "id": "route/synthetic_ascent",
                    "from_member": "synthetic_interior",
                    "from": {"x": 1, "y": 1},
                    "to_member": "synthetic_surface",
                    "to": {"x": 3, "y": 0},
                    "direction": "up",
                },
                {
                    "id": "route/synthetic_descent",
                    "from_member": "synthetic_surface",
                    "from": {"x": 3, "y": 0},
                    "to_member": "synthetic_interior",
                    "to": {"x": 1, "y": 1},
                    "direction": "down",
                },
            ]
        },
    }
    write(out / PROJECTION_PATH, json_bytes(projection_document))

    projection = load(out, PROJECTION_PATH)
    member = projection.member("synthetic_surface")

    manifest = {
        "schema_version": 1,
        "kind": MANIFEST_KIND,
        "workbench_version": VERSION,
        "session_id": SESSION_ID,
        "opened_at": STAMP,
        "view": "logical",
        "land_id": projection.land_id,
        "realm_id": projection.realm_id,
        "candidate_member": projection.candidate_member,
        "repository_revision": REVISION,
        "revision_binding": "advisory",
        "digest_binding": "fail_closed",
        "projection": {"path": projection.path, "sha256": projection.digest},
        "base_digests": projection.source_records(),
        "authority": {
            "tracked_content": False,
            "runtime_input": False,
            "staged_operations": False,
            "apply": False,
        },
        "retention": {"policy": "disposable", "statement": RETENTION_STATEMENT},
    }
    write(out / "session/manifest.json", json_bytes(manifest))

    # One packet per gesture, so every gesture has a tracked example an agent
    # (or a reviewer) can read without running anything.
    plans = [
        ("sel-0001", "click", {"cell": {"x": 1, "y": 1}},
         "the landmark and the route both live here"),
        ("sel-0002", "box", {"rect": {"x": 2, "y": 1, "width": 2, "height": 1}},
         "the whole hall footprint"),
        ("sel-0003", "lasso",
         {"polygon": [{"x": 0.0, "y": 0.0}, {"x": 2.0, "y": 0.0},
                      {"x": 2.0, "y": 2.0}, {"x": 0.0, "y": 2.0}]},
         "the north-west quarter"),
        ("sel-0004", "paint",
         {"cells": [{"x": 2, "y": 2}, {"x": 3, "y": 2}]},
         "the ground the hall opens onto"),
    ]

    operations = []
    for index, (selection_id, gesture, payload, comment) in enumerate(plans, start=1):
        cells = cells_for_gesture(member, gesture, payload)
        packet = build(
            projection=projection,
            member=member,
            gesture=gesture,
            cells=cells,
            screen_region={"x": 0, "y": 0, "width": 64, "height": 48},
            comment=comment,
            selection_id=selection_id,
            created_at=STAMP,
            repository_revision=REVISION,
            mask_reference=None,
            geometry=payload,
        )
        if gesture in ("lasso", "paint"):
            mask_relative = f"session/masks/{selection_id}.pbm"
            digest = write(out / mask_relative, mask_bytes(member, cells))
            packet["screen_region"]["mask"] = {
                "path": mask_relative,
                "sha256": digest,
                "format": "pbm_p1_over_cell_bounds",
            }
        write(out / f"session/selections/{selection_id}.json", json_bytes(packet))
        operations.append(
            {
                "schema_version": 1,
                "kind": SELECTION_RECORD,
                "record_id": f"op-{index * 2 - 1:04d}",
                "recorded_at": STAMP,
                "author": "owner",
                "selection_id": selection_id,
                "packet": f"selections/{selection_id}.json",
                "operation": None,
            }
        )
        operations.append(
            {
                "schema_version": 1,
                "kind": COMMENT_RECORD,
                "record_id": f"op-{index * 2:04d}",
                "recorded_at": STAMP,
                "author": "owner",
                "selection_id": selection_id,
                "comment": comment,
                "operation": None,
            }
        )

    write(
        out / "session/operations.jsonl",
        "".join(json.dumps(record) + "\n" for record in operations).encode("utf-8"),
    )
    print(f"wrote the synthetic session fixture under {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
