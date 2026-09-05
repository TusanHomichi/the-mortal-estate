#!/usr/bin/env python3
"""Prove server sign-in, admission, observed land, and individual cooldowns over TLS.

Uses a scratch database and the existing production-smoke wire transport.
This proves the server boundary; it does not claim browser UI integration.
"""

from __future__ import annotations

import argparse
import re
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from live_server_harness import (  # noqa: E402
    LiveServer,
    ProofError,
    World,
    read_admin_url,
)

from live_wire_client import LiveWireClient
from run_production_smoke import SmokeError
from logout_proof import prove_disconnected_logout

SUCCESS_SENTINEL = "TME_SERVER_LIVE_PROOF_OK"

# Owner ruling September 5: every accepted move/wait gets its own full duration.
STANDARD_ACTION_MSEC = 3000
COOLDOWN_TOLERANCE_MSEC = 750
COOLDOWN_OBSERVATION = re.compile(r"^cooldown_observation = start (\d+) ready (\d+) elapsed (\d+) ms$", re.MULTILINE)
OBSERVATION_CENTRE = re.compile(r"^observer = Observation centre (\d+),(\d+) ", re.MULTILINE)
PROOF_WORLD_DOCUMENT = "content/lands/identity-proof/world.json"


def proof(arguments: argparse.Namespace) -> int:
    world = World.declared(PROOF_WORLD_DOCUMENT, key="identity-proof-world")
    with LiveServer(read_admin_url(arguments.admin_url_file), world, keep=arguments.keep) as server:
        with LiveWireClient(server, timeout=arguments.timeout) as client:
            centre = client.frame["observation_center"]["position"]
            check_land(f"observer = Observation centre {centre['x']},{centre['y']} ", world, server.run_directory)
            observations = []
            for index, offset in enumerate([0.137, 1.173, 2.511]):
                client.wait_for(lambda frame: frame["can_act"])
                time.sleep(offset)
                began = time.monotonic()
                result, _ = client.command({"kind": "wait"})
                if result.get("disposition") != {"kind": "accepted"}:
                    raise ProofError("the server did not accept a ready action")
                frame = client.wait_for(lambda frame: not frame["can_act"])
                started, ready = int(frame["logical_time"]), int(frame["ready_at"])
                rejected, _ = client.command({"kind": "wait"})
                if rejected.get("disposition", {}).get("kind") != "rejected":
                    raise ProofError("the server accepted another action during cooldown")
                if int(client.frame["ready_at"]) != ready:
                    raise ProofError("a rejected action changed the deadline")
                if index == 0:
                    client.gameplay.close()
                    client.gameplay = client.session.connect()
                    if client.frame["can_act"] or int(client.frame["ready_at"]) != ready:
                        raise ProofError("reconnection cleared or rescheduled the cooldown")
                client.wait_for(lambda frame: frame["can_act"])
                elapsed = round((time.monotonic() - began) * 1000)
                observations.append(f"cooldown_observation = start {started} ready {ready} elapsed {elapsed} ms")
            report = "\n".join(observations)
            print(report)
            check_cooldowns(report)
            cookie = client.session.cookie
        client.public.request("GET", "/v3/session", cookie=cookie, expected=(401,))
        prove_disconnected_logout(server)
        print("sign-in, admission, cooldown rejection, reconnect, and logout passed")
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
    parser.add_argument("--timeout", type=float, default=300.0, help="seconds allowed for a wire response")
    parser.add_argument("--keep", action="store_true", help="keep the scratch database and run directory")
    arguments = parser.parse_args()
    try:
        return proof(arguments)
    except (ProofError, SmokeError, OSError) as error:
        print(f"live proof failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
