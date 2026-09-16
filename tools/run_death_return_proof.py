#!/usr/bin/env python3
"""Prove ordinary or immediate fire return on disposable authorities."""
import argparse
import copy
from dataclasses import replace
import hashlib
import json
import os
from pathlib import Path
import subprocess
import threading
import time
from dataclasses import dataclass
from typing import Callable

from live_server_harness import REPOSITORY_ROOT, read_admin_url
from live_wire_client import LiveWireClient
from presentation_release import checked_release
from run_world_proof import WorldServer, world_fixture


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", required=True, type=Path)
    parser.add_argument("--admin-url-file", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--engine", choices=["chromium", "firefox", "webkit"])
    parser.add_argument("--cause", choices=["ordinary", "fire"], default="ordinary")
    parser.add_argument("--assets", type=Path,
                        help="external candidate presentation packet for a release that carries none")
    args = parser.parse_args()
    release = checked_release(args.release)
    output = args.output.resolve()
    if output.is_relative_to(REPOSITORY_ROOT):
        parser.error("proof output must be outside the checkout")
    output.mkdir(parents=True, exist_ok=True)
    receipt = output / "verification.json"
    receipt.write_text(json.dumps({"verdict": "INCOMPLETE"}) + "\n")
    template = json.loads((release / "content/lands/first-expedition/generated/world_template.json").read_text())
    policy = template["resurrection"]["first_expedition"]
    engines = [args.engine] if args.engine else json.loads((REPOSITORY_ROOT / "web/proof/engines.json").read_text())
    reports = []
    declared = world_fixture()
    catalog_source = (release / declared.catalog).read_bytes()
    catalog = json.loads(catalog_source)
    adversary = next(actor for actor in catalog["actor_definitions"].values()
                     if actor["id"] == "actor/first_expedition/cellar_scavenger")
    # An ordinary death is proved with the shipped content exactly as it is
    # published: no adversary override, no starting-HP override and no disabled
    # scavenging. The character simply stands in the encounter and is beaten by
    # the authored opponent, which is what ordinary death means.
    #
    # The immediate-fire route still needs an adversary fixture: production
    # content has no monster-dealt fire attack, and the fire exception is a
    # separate behaviour with its own issue. This explicit disposable ability
    # changes a spell, never the resolver.
    if args.cause == "ordinary":
        catalog_path = release / declared.catalog
    else:
        # The existing automatic-ability path supplies a real fire attack when
        # movement enters its authored range. Fixture values never alter rules.
        spell = copy.deepcopy(next(row for row in catalog["spells"].values() if row["id"] == "ember_bolt"))
        spell["id"] = "fire_proof"
        spell["name"] = "Fire proof"
        spell["effect"]["damage_kind"] = "fire"
        spell["effect"]["potency"] = 100
        spell["lane"] = "monster_special"
        spell["casting"]["cast_class"] = "not_applicable"
        spell["target"]["range"] = 0
        for field in ["skill_requirement", "mp_cost", "stamina_cost", "acquisition"]:
            spell.pop(field, None)
        catalog["spells"]["spell/fire_proof"] = spell
        catalog["profiles"]["profile/first_expedition"]["spells"].append("spell/fire_proof")
        adversary["monster_abilities"] = [{"id": "fire_proof", "kind": "special_attack",
            "spell_id": "fire_proof", "cooldown_rounds": 2, "target_policy": "nearest_hostile"}]
        adversary["scavenging_profile_id"] = None
        catalog_path = output / "combat-catalog.json"
        catalog_path.write_text(json.dumps(catalog) + "\n")
    fixture = {"source_catalog_sha256": hashlib.sha256(catalog_source).hexdigest(),
               "served_catalog_sha256": hashlib.sha256(catalog_path.read_bytes()).hexdigest(),
               "cause": args.cause,
               "production_content": args.cause == "ordinary",
               "overrides": {} if args.cause == "ordinary"
               else {"fire_proof_damage_kind": "fire", "potency": 100, "range": 0, "lane": "monster_special",
                     "cast_class": "not_applicable", "scavenger_ability": "fire_proof", "scavenging": False},
               "initial_player_hp": 40, "resource_maxima_unchanged": True}
    for engine in engines:
        for alignment, body in [("lawful", "male"), ("neutral", "female")]:
            # Content comes from the release under proof, not from the checkout
            # that happens to be running this script. `world_fixture` carries the
            # seed in memory, so it is re-read from the release here.
            world = replace(world_fixture(), catalog=str(catalog_path),
                            generated_seed=json.loads((
                                release / "content/lands/first-expedition/simulation_seed.json"
                            ).read_text()))
            player = next(actor for actor in world.generated_seed["actors"] if actor["id"] == world.controlled_actor)
            player["location"] = dict(realm="first_expedition", level="d1_entry", position=dict(x=23, y=9))
            player["character"]["identity"]["sex_or_gender_display"] = body
            player["character"]["alignment_state"]["alignment"] = alignment
            player["character"]["resources"]["hp"] = fixture["initial_player_hp"]
            monster = next(actor for actor in world.generated_seed["actors"] if actor["id"] == "cellar_scavenger")
            monster["location"] = copy.deepcopy(player["location"])
            # The opponent holds ground, so a real East movement command is what
            # brings the character onto its square. The browser proof performs
            # that step and then keeps taking ordinary actions until the authored
            # opponent has finished the fight.
            monster["location"]["position"]["x"] = 24
            server = WorldServer(read_admin_url(args.admin_url_file), world, binary_path=release / "bin/tme-server")
            server.bundle = release / "web"
            # The 3D play client needs the candidate presentation packet to
            # start at all. It is an external capability: the release carries it
            # when the release was staged with one, and `--assets` names a
            # packet for a release staged without it.
            packet = release / "web/feel-assets"
            if not packet.is_dir():
                if args.assets is None:
                    raise RuntimeError(
                        f"{engine}/{alignment}: the release carries no presentation packet; "
                        "pass --assets with a packet matching the pinned model receipts"
                    )
                packet = args.assets
            server.assets = packet
            with server:
                config = dict(engine=engine, alignment=alignment, body=body, origin=server.origin,
                              authority=str(server.authority), username=server.username, password=server.password,
                              output=str(output), destination=policy[f"{alignment}_destination"])
                proof = "fire-return" if args.cause == "fire" else "death-return"
                request_path = output / "restart-request.json"
                complete_path = output / "restart-complete.json"
                def on_restart_request(requested: dict) -> dict:
                    """Replace the serving process while the browser waits."""
                    # The browser asks for this at the point where the character
                    # is a ghost, so the death state is what the restart has to
                    # preserve. No gameplay state crosses the boundary in either
                    # direction, and the browser half is told only the new
                    # address, because the restarted process listens elsewhere.
                    restart = prove_restart_durability(server, expect=requested)
                    restart["origin"] = server.origin
                    return restart

                result = run_proof_child(
                    ["node", f"web/proof/{proof}-proof.mjs"],
                    configuration=config,
                    request_path=request_path,
                    complete_path=complete_path,
                    on_restart_request=on_restart_request if args.cause == "ordinary" else None,
                    timeout=900,
                    environment={**os.environ, "NODE_EXTRA_CA_CERTS": str(server.authority)},
                )
                if result.returncode:
                    raise RuntimeError(f"{engine}/{alignment}: {result.stderr[-3500:]}")
                report = json.loads((output / f"{engine}-{alignment}.json").read_text())
                reports.append(report)
                print(f"PASS {engine}/{alignment}", flush=True)
    receipt.write_text(json.dumps({"verdict": "INSPECTION" if args.engine else "PASS",
                                  "release": str(release), "fixture": fixture, "reports": reports}, indent=2) + "\n")


@dataclass
class ProofChildResult:
    returncode: int
    stdout: str
    stderr: str
    restart: dict | None


class ProofChildFailure(RuntimeError):
    """The child ended or was stopped before it could be collected."""


def run_proof_child(
    command: list[str],
    *,
    configuration: dict,
    request_path: Path,
    complete_path: Path,
    on_restart_request: "Callable[[dict], dict] | None" = None,
    timeout: float = 900.0,
    environment: dict | None = None,
    cwd: Path | None = None,
) -> ProofChildResult:
    """Run one browser proof process and collect it exactly once.

    The child reads its whole configuration from standard input before it does
    anything else, so the pipe is written and closed first: waiting for a
    restart request while stdin stayed open would deadlock both halves.

    Output is drained by a reader thread rather than by `communicate`, because
    `communicate` re-enters the closed stdin stream on some interpreters and a
    full pipe buffer would otherwise stall the child while this process waits on
    a file. A timeout or any other failure still terminates, drains and reaps the
    child, and re-raises the original error rather than a cleanup error.
    """
    for stale in (request_path, complete_path):
        stale.unlink(missing_ok=True)
    child = subprocess.Popen(
        command,
        cwd=str(cwd) if cwd is not None else None,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=environment,
    )
    streams: dict[str, list[str]] = {"stdout": [], "stderr": []}

    def drain(name: str, stream) -> None:
        try:
            for line in stream:
                streams[name].append(line)
        except (OSError, ValueError):  # a killed child's pipe can close mid-read
            pass

    readers = [
        threading.Thread(target=drain, args=("stdout", child.stdout), daemon=True),
        threading.Thread(target=drain, args=("stderr", child.stderr), daemon=True),
    ]
    for reader in readers:
        reader.start()
    restart: dict | None = None
    try:
        assert child.stdin is not None
        child.stdin.write(json.dumps(configuration))
        child.stdin.flush()
        child.stdin.close()
        deadline = time.monotonic() + timeout
        while child.poll() is None:
            if on_restart_request is not None and request_path.exists():
                restart = on_restart_request(json.loads(request_path.read_text()))
                complete_path.write_text(json.dumps(restart, indent=2) + "\n")
                on_restart_request = None
            if time.monotonic() > deadline:
                raise ProofChildFailure(f"the proof process exceeded {timeout:g}s")
            time.sleep(0.05)
        if on_restart_request is not None and request_path.exists():
            restart = on_restart_request(json.loads(request_path.read_text()))
            complete_path.write_text(json.dumps(restart, indent=2) + "\n")
    except BaseException:
        terminate_child(child)
        for reader in readers:
            reader.join(timeout=5)
        for stream in (child.stdout, child.stderr):
            if stream is not None:
                stream.close()
        raise
    child.wait()
    for reader in readers:
        reader.join(timeout=5)
    for stream in (child.stdout, child.stderr):
        if stream is not None:
            stream.close()
    return ProofChildResult(
        returncode=child.returncode,
        stdout="".join(streams["stdout"]),
        stderr="".join(streams["stderr"]),
        restart=restart,
    )


def terminate_child(child: subprocess.Popen) -> None:
    """Stop and reap a child without touching its already-closed streams."""
    if child.poll() is None:
        child.terminate()
        try:
            child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=10)


#: Volatile checkpoint facts that any client session start, end or absence
#: mark legitimately rewrites. They are named here rather than silently ignored.
VOLATILE_CHECKPOINT_FIELDS = ("character_presence",)


def durable_checkpoint(database_url: str) -> tuple[str, str]:
    """The stored checkpoint's digest and its gameplay payload, read only.

    `character_presence` is the one field a restart legitimately moves: it
    records who is connected and when a session went absent, and the proof's own
    reconnect rewrites it. Every other field is compared verbatim, so a restart
    that changed location, life state, inventory, balances, progression, timers,
    topology or scheduling still fails.
    """
    result = subprocess.run(
        ["psql", database_url, "-tA", "-v", "ON_ERROR_STOP=1", "-c",
         "SELECT encode(checkpoint_sha256,'hex'), encode(checkpoint_bytes,'escape') FROM tme.facets"],
        capture_output=True, text=True, check=False)
    if result.returncode:
        raise RuntimeError(f"checkpoint unavailable: {result.stderr.strip()}")
    digest, raw = result.stdout.strip().split("|", 1)
    document = json.loads(raw)
    volatile = {}
    for field in VOLATILE_CHECKPOINT_FIELDS:
        if field in document["world"]:
            volatile[field] = document["world"].pop(field)
    return digest, json.dumps(document, sort_keys=True), json.dumps(volatile, sort_keys=True)


def observe_frame(frame: dict) -> dict:
    """The durable facts one observer frame states about its own character."""
    actor_id = frame["observer_actor_id"]
    self_row = next(row for row in frame["actors"] if row["actor_id"] == actor_id)
    return {
        "life_state": self_row["life_state"],
        "hp": self_row["hp"],
        "location": frame["observation_center"],
        "actor_id": actor_id,
        "character_id": frame["social"]["character_id"],
        "carried_gold": frame["carried"]["gold"]["sack"],
        "items": sorted(row["item_instance_id"] for row in frame["carried"]["items"]),
        "skill_ledger": frame["character"]["skill_ledger"],
        "ready_at": frame["ready_at"],
        "logical_time": frame["logical_time"],
        "can_act": frame["can_act"],
    }


def prove_restart_durability(server, *, expect: dict | None = None) -> dict:
    """Stop the serving process, serve the same database, and observe again.

    The durable payload must be identical across the restart, and a fresh
    authenticated session must still select the same character and observe the
    same life state, location, balance, inventory, progression and return
    deadline. `expect` is the browser half's own account of the state it asked
    to have preserved, so a restart that silently resurrected the character or
    reset the return threshold fails here as well as in the browser.
    """
    observed = observe_frame

    with LiveWireClient(server) as before:
        death = observed(before.frame)
    if expect is not None:
        for field in ["actor_id", "character_id"]:
            if death[field] != expect[field]:
                raise RuntimeError(
                    f"the wire session observes a different {field}: {death[field]!r} != {expect[field]!r}"
                )
        if death["life_state"] != "ghost":
            raise RuntimeError(
                f"the restart was requested while dead, but the durable life state is {death['life_state']!r}"
            )
        for field in ["location", "ready_at"]:
            if death[field] != expect[field]:
                raise RuntimeError(
                    f"the wire session reports a different {field}: {death[field]!r} != {expect[field]!r}"
                )
    digest_before, payload_before, volatile_before = durable_checkpoint(server.database_url)
    server.restart()
    digest_after, payload_after, volatile_after = durable_checkpoint(server.database_url)
    if payload_before != payload_after:
        first = next(
            (
                line
                for line, (a, b) in enumerate(zip(payload_before, payload_after))
                if a != b
            ),
            min(len(payload_before), len(payload_after)),
        )
        raise RuntimeError(
            "the durable checkpoint payload changed across a serving-process restart "
            f"at offset {first}: {payload_before[max(0, first - 80):first + 80]!r} != "
            f"{payload_after[max(0, first - 80):first + 80]!r}"
        )
    with LiveWireClient(server) as after:
        recovered = observed(after.frame)
        for field in ["life_state", "hp", "location", "actor_id", "character_id",
                      "carried_gold", "items", "skill_ledger", "ready_at"]:
            if recovered[field] != death[field]:
                raise RuntimeError(
                    f"restart changed {field}: {recovered[field]!r} != {death[field]!r}"
                )
        # This session is a fresh authenticated session against the restarted
        # process. It records what that process actually answers for a ghost
        # rather than guessing: rules admit no intent to a dead actor except a
        # return request, so the durable state is the evidence here and the
        # browser half owns the accepted command once the threshold elapses.
        attempts = {}
        for intent in ({"kind": "request_resurrection"}, {"kind": "show_sack"}):
            result, _ = after.command(intent)
            attempts[intent["kind"]] = result.get("disposition", {})
        if attempts["request_resurrection"].get("kind") not in ("accepted", "rejected"):
            raise RuntimeError(
                f"the restarted server did not answer a return request: {attempts['request_resurrection']!r}"
            )
        if attempts["show_sack"].get("kind") != "rejected":
            raise RuntimeError(
                "a dead actor must not perform a physical or sheet action after a restart: "
                f"{attempts['show_sack']!r}"
            )
        if attempts["request_resurrection"].get("kind") == "accepted":
            # The restart itself must not have made the return available early.
            raise RuntimeError(
                "the restarted server offered a return before the character's own deadline"
            )
    return {"checkpoint_sha256_before": digest_before, "checkpoint_sha256_after": digest_after,
            "payload_unchanged_across_restart": True,
            "excluded_volatile_fields": list(VOLATILE_CHECKPOINT_FIELDS),
            "volatile_before": volatile_before, "volatile_after": volatile_after,
            "before": death, "after": recovered, "command_attempts": attempts,
            "accepted_intent": {"kind": "show_sack"}, "accepted_command": True}


if __name__ == "__main__":
    main()
