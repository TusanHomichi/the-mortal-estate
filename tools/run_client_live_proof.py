#!/usr/bin/env python3
"""Prove the client against a real server, from an empty database.

This is the phase-6 stop point made runnable: a scratch PostgreSQL database, the
schema, one enrolled account, one bootstrapped character, the real `tme-server`
binary, a TLS front the client can trust, and the shipped Godot client scene
driven through sign-in, admission, authoritative play, and sign-out.

The provisioning lives in `tools/live_server_harness.py`, which this shares with
`tools/run_fixture_land_capture.py`. What remains here is this proof's own
choices: which world it serves, which client script it drives, and what counts
as success.

Usage:

    tools/run_client_live_proof.py --admin-url-file <path> [--godot <path>]

Everything else is derived. `--keep` leaves the database and the run directory
in place for inspection.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
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

SUCCESS_SENTINEL = "TME_CLIENT_LIVE_PROOF_OK"
CLIENT_SCRIPT = "res://tests/live_server_play.gd"

# Owner ruling September 5: every accepted move/wait gets its own full duration.
STANDARD_ACTION_MSEC = 3000
COOLDOWN_TOLERANCE_MSEC = 750
COOLDOWN_OBSERVATION = re.compile(r"^cooldown_observation = start (\d+) ready (\d+) elapsed (\d+) ms$", re.MULTILINE)
OBSERVATION_CENTRE = re.compile(r"^shell = Observation centre (\d+),(\d+) ", re.MULTILINE)
PROOF_WORLD_DOCUMENT = "content/lands/identity-proof/world.json"


def proof(arguments: argparse.Namespace) -> int:
    godot = resolve_godot(arguments.godot)
    world = World.declared(PROOF_WORLD_DOCUMENT, key="identity-proof-world")
    print(f"world: {world.world_template}")
    with LiveServer(
        read_admin_url(arguments.admin_url_file), world, keep=arguments.keep
    ) as server:
        print("--- client ---", flush=True)
        client = server.run_client(godot, CLIENT_SCRIPT, timeout=arguments.timeout)
        emit_client_output(client)
        if client.returncode != 0 or SUCCESS_SENTINEL not in client.stdout:
            raise ProofError(f"the client proof failed with status {client.returncode}")
        print("--- the land the client is standing in ---")
        check_land(client.stdout, world, server.run_directory)
        print("--- individual cooldowns ---")
        check_cooldowns(client.stdout)
        print("--- server log tail ---")
        print(server.log_tail())
        print(SUCCESS_SENTINEL)
        return 0


def check_land(stdout: str, world: World, run_directory) -> None:
    """Judge WHERE the client is, not merely that it arrived somewhere.

    The client reports the observation centre it is presenting. The served
    world's own seed says where the controlled actor stands. If the runtime were
    serving some other land — a fixture, a corpus scenario, anything the tree
    happens to carry — those two would not agree, and this proof would be
    reporting a sign-in into a world nobody asked for.
    """
    observed = OBSERVATION_CENTRE.search(stdout)
    if observed is None:
        raise ProofError("the client reported no observation centre to judge")
    centre = (int(observed.group(1)), int(observed.group(2)))
    seed = world.seed_document(run_directory)
    actors = {actor["id"]: actor for actor in seed["actors"]}
    controlled = actors.get(world.controlled_actor)
    if controlled is None:
        raise ProofError(
            f"the served seed carries no actor {world.controlled_actor!r} to stand on"
        )
    expected = (
        controlled["location"]["position"]["x"],
        controlled["location"]["position"]["y"],
    )
    level = controlled["location"]["level"]
    print(f"observation centre {centre[0]},{centre[1]} on {seed['id']}/{level}")
    if centre != expected:
        raise ProofError(
            f"the client is presenting {centre[0]},{centre[1]}; the served world seats "
            f"{world.controlled_actor} at {expected[0]},{expected[1]} on {level}"
        )
    others = sorted(id for id in actors if id != world.controlled_actor)
    print(f"the seed stands {len(others)} other cast member(s) in this land: {', '.join(others)}")


def check_cooldowns(stdout: str) -> None:
    observations = [tuple(map(int, row)) for row in COOLDOWN_OBSERVATION.findall(stdout)]
    if len(observations) < 3:
        raise ProofError("at least three offset action cooldowns must be observed")
    for started, ready, elapsed in observations:
        if ready - started != STANDARD_ACTION_MSEC:
            raise ProofError("an action did not receive its complete individual cooldown")
        if abs(elapsed - STANDARD_ACTION_MSEC) > COOLDOWN_TOLERANCE_MSEC:
            raise ProofError("observed readiness did not follow the individual deadline")
    if len({started % STANDARD_ACTION_MSEC for started, _, _ in observations}) < 2:
        raise ProofError("the actions were aligned to a shared phase")
    print(f"{len(observations)} complete individual action cooldowns observed")


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
    parser.add_argument("--timeout", type=float, default=300.0, help="seconds allowed for the client run")
    parser.add_argument("--keep", action="store_true", help="keep the scratch database and run directory")
    arguments = parser.parse_args()
    if not arguments.godot:
        parser.error("--godot or TME_GODOT must name the pinned Godot binary")
    try:
        return proof(arguments)
    except ProofError as error:
        print(f"live proof failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
