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

#: Owner ruling D5 (2026-08-19): one authoritative gameplay pulse at 3.0 seconds.
#:
#: The runtime constant is `GAMEPLAY_PULSE` in
#: `crates/tme-server/src/scheduler.rs`. This proof deliberately restates the
#: ruled value instead of importing it: an end-to-end proof that reads the very
#: constant it is proving would only establish that the constant equals itself.
#: What is checked here is that a real client, against a real server, observes
#: the beat the ruling promises.
RULED_PULSE_MSEC = 3000.0

#: Scheduling slack allowed between two observed beats on a real host — client
#: frame quantisation, TLS, and database commit time. Wide enough that ordinary
#: jitter never fails the proof, far narrower than the distance to any other
#: candidate cadence.
PULSE_TOLERANCE_MSEC = 750.0

#: `pulse_observation = T<logical time> at <wall clock ms> ms`, as printed by
#: `client/tests/live_server_play.gd`.
PULSE_OBSERVATION = re.compile(r"^pulse_observation = T(\d+) at (\d+) ms$", re.MULTILINE)

#: `shell = Observation centre <x>,<y> ...`, as printed by the same script from
#: the view's own status line.
OBSERVATION_CENTRE = re.compile(r"^shell = Observation centre (\d+),(\d+) ", re.MULTILINE)

#: The land this proof serves: the identity proof's, compiled by the authoring
#: compiler and declared by the land itself. Naming the document rather than the
#: files is what keeps this proof and the tree from disagreeing about which land
#: the runtime loads.
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
        print("--- authoritative pulse ---")
        check_pulse(client.stdout)
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


def check_pulse(stdout: str) -> None:
    """Judge the observed beats against ruling D5.

    The client reports one line per authoritative beat: the frame's own logical
    time and the wall-clock millisecond it arrived. Logical time must advance by
    exactly one round per beat, and the wall-clock gap between beats must be the
    ruled pulse within tolerance. A one-second cadence fails this by two full
    seconds, which is the regression this proof exists to catch.
    """
    beats = [
        (int(logical), int(wall)) for logical, wall in PULSE_OBSERVATION.findall(stdout)
    ]
    if len(beats) < 2:
        raise ProofError(
            f"the client reported {len(beats)} authoritative beat(s); "
            "at least two are needed to observe a cadence"
        )
    failures = []
    for (before_logical, before_wall), (after_logical, after_wall) in zip(beats, beats[1:]):
        interval = after_wall - before_wall
        drift = interval - RULED_PULSE_MSEC
        verdict = "ok" if abs(drift) <= PULSE_TOLERANCE_MSEC else "OUT OF BAND"
        print(
            f"T{before_logical} -> T{after_logical} in {interval} ms "
            f"({drift:+.0f} ms from the ruled {RULED_PULSE_MSEC:.0f} ms) {verdict}"
        )
        if after_logical != before_logical + 1:
            failures.append(
                f"logical time went T{before_logical} -> T{after_logical}; "
                "one beat must advance exactly one round"
            )
        if abs(drift) > PULSE_TOLERANCE_MSEC:
            failures.append(
                f"T{before_logical} -> T{after_logical} took {interval} ms, "
                f"outside {RULED_PULSE_MSEC:.0f} +/- {PULSE_TOLERANCE_MSEC:.0f} ms"
            )
    if failures:
        raise ProofError(
            "the observed cadence contradicts ruling D5: " + "; ".join(failures)
        )
    print(
        f"{len(beats)} beats observed at the ruled {RULED_PULSE_MSEC / 1000:.1f} s pulse"
    )


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
