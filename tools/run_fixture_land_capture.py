#!/usr/bin/env python3
"""Take a gameplay capture over the authoring fixture's land, from a real server.

This is the **accuracy reference** for the Workbench's capture path, not its
ordinary route. It provisions a scratch database, serves the compiled authoring
fixture — the same land the Workbench's logical projection is compiled from —
drives the shipped `ClientRoot.tscn` through sign-in and admission, and captures
the real frame the real server sent.

It exists so that the cheap route can be checked against it. The cheap route
(`tools/workbench/capture.py`) replays a recorded frame in seconds; this one
takes minutes and a database. Their sidecars must resolve to identical
addresses, and `tests/test_capture_addressing.py` asserts exactly that against
the recorded pair.

It also **records the frame** the server sent, which is how the tracked fixture
frame the cheap route replays comes into existence. Re-record it with
`--record-frame` whenever the fixture land or the frame contract changes; a
capture is never invented here.

A real display is required. Godot's headless display driver produces no
viewport image, so the client runs under `xvfb-run`.

Usage:

    tools/run_fixture_land_capture.py \\
        --admin-url-file <postgres superuser url file> \\
        --godot <pinned godot binary> \\
        --output <directory> [--record-frame <path>]
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from live_server_harness import (  # noqa: E402
    GODOT_VERSION,
    LiveServer,
    ProofError,
    World,
    emit_client_output,
    read_admin_url,
    resolve_godot,
)

SUCCESS_SENTINEL = "TME_CAPTURE_OK"
CLIENT_SCRIPT = "res://tests/live_capture.gd"
WINDOW = (1024, 768)
XVFB_SCREEN = "1280x1024x24"

#: The compiled authoring fixture, as the runtime loads it. This is the whole
#: point of this driver: the Workbench's logical projection and this world
#: template are two emissions of one compiler run over one master, so a capture
#: taken here addresses the same lattice a logical selection does.
FIXTURE_WORLD_TEMPLATE = "content/authoring-fixture/generated/world_template.json"

#: The fixture land has no tracked simulation seed, because the corpus seeds
#: describe the corpus land's levels and this land has two of its own. The seed
#: is therefore generated for the run: one controlled actor, standing on the
#: fixture's arrival square, and nothing else. It is proof-harness input in the
#: same class as the bootstrap manifest — never content, never tracked, and
#: never read by anything but this run.
FIXTURE_SEED = {
    "schema_version": 3,
    "kind": "simulation_seed",
    "id": "authoring_fixture",
    "actors": [
        {
            "active_effects": [],
            "actor_definition_id": "actor/first_land_structure/player",
            "carried": {"gold": {"left_hand": 0, "right_hand": 0, "sack": 0}, "items": []},
            "character": {
                "alignment_state": {"alignment": "lawful", "karma_points": 0},
                "attributes": {
                    "charisma": 10,
                    "constitution": 10,
                    "dexterity": 10,
                    "intelligence": 14,
                    "strength": 10,
                    "wisdom": 12,
                },
                "identity": {
                    "base_class_id": "wizard",
                    "current_class_id": "wizard",
                    "display_class": "Wizard",
                    "nationality_id": "aldland",
                },
                "known_spells": [
                    {"lane": "wizard_magic", "learned_at_level": 1, "spell_id": "sense_secret"}
                ],
                "physical_attribute_adds": {"dexterity_adds": 0, "strength_adds": 0},
                "progression": {"experience": 0, "level": 1},
                "promotion_history": [],
                "resources": {
                    "hp": 20,
                    "max_hp": 20,
                    "max_mp": 40,
                    "max_stamina": 20,
                    "mp": 40,
                    "peak_hp": 20,
                    "stamina": 20,
                },
                "skill_ledger": [
                    {
                        "critique_rank": 0,
                        "learning_rate": 1,
                        "level": 1,
                        "practice_points": 0,
                        "track_id": "wizard_magic",
                    }
                ],
            },
            "character_id": "character:authoring_fixture:primary",
            "id": "player",
            # The fixture's arrival landmark, which the compiler places at the
            # dock. Standing the controlled actor anywhere else would make the
            # capture describe a region the arrival does not reach.
            "location": {"level": "surface", "position": {"x": 12, "y": 14}, "realm": "testland"},
            "npc": None,
        }
    ],
    "item_instances": {},
    "ground_items": [],
    "service_instances": [],
    "merchant_inventories": [],
    "ecology_sites": [],
}

FIXTURE_LAND = World(
    world_template=FIXTURE_WORLD_TEMPLATE,
    generated_seed=FIXTURE_SEED,
    key="fixture-land-capture",
)


def capture(arguments: argparse.Namespace) -> int:
    godot = resolve_godot(arguments.godot)
    if shutil.which("xvfb-run") is None:
        raise ProofError(
            "xvfb-run is required: a capture needs a real display, and Godot's "
            "headless display driver produces no viewport image"
        )
    output = Path(arguments.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    frame_record = output / "recorded-frame.json"

    started = time.monotonic()
    with LiveServer(
        read_admin_url(arguments.admin_url_file), FIXTURE_LAND, keep=arguments.keep
    ) as server:
        provisioned = time.monotonic()
        print("--- client ---", flush=True)
        client = server.run_client(
            godot,
            CLIENT_SCRIPT,
            extra_environment={
                "TME_CAPTURE_OUTPUT": str(output),
                "TME_CAPTURE_FRAME_OUT": str(frame_record),
            },
            timeout=arguments.timeout,
            display=["xvfb-run", "-a", "--server-args", f"-screen 0 {XVFB_SCREEN}"],
            window=WINDOW,
        )
        emit_client_output(client)
        if client.returncode != 0 or SUCCESS_SENTINEL not in client.stdout:
            raise ProofError(f"the live capture failed with status {client.returncode}")
        finished = time.monotonic()
        print("--- server log tail ---")
        print(server.log_tail())

    timings = {
        "provision_seconds": round(provisioned - started, 3),
        "client_capture_seconds": round(finished - provisioned, 3),
        "total_seconds": round(finished - started, 3),
    }
    (output / "timings.json").write_text(json.dumps(timings, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(timings, indent=2))

    if arguments.record_frame:
        destination = Path(arguments.record_frame).resolve()
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(frame_record.read_bytes())
        print(f"recorded frame: {destination}")
    print(SUCCESS_SENTINEL)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--admin-url-file",
        required=True,
        help="file holding a PostgreSQL superuser URL used only to create and drop the scratch database",
    )
    parser.add_argument(
        "--godot",
        default=os.environ.get("TME_GODOT", ""),
        help="Godot binary, pinned to " + GODOT_VERSION,
    )
    parser.add_argument("--output", required=True, help="directory to write the capture into")
    parser.add_argument(
        "--record-frame",
        default=None,
        help="also copy the authoritative frame this run received to this path",
    )
    parser.add_argument("--timeout", type=float, default=300.0, help="seconds allowed for the client run")
    parser.add_argument("--keep", action="store_true", help="keep the scratch database and run directory")
    arguments = parser.parse_args()
    if not arguments.godot:
        parser.error("--godot or TME_GODOT must name the pinned Godot binary")
    try:
        return capture(arguments)
    except ProofError as error:
        print(f"live capture failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
