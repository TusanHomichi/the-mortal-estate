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
            # The presentation packet is an optional external capability. Absent,
            # the client serves its carried bundle and the proof records that no
            # candidate artwork was bound rather than claiming one.
            packet = release / "web/feel-assets"
            server.assets = packet if packet.is_dir() else None
            with server:
                config = dict(engine=engine, alignment=alignment, body=body, origin=server.origin,
                              authority=str(server.authority), username=server.username, password=server.password,
                              output=str(output), destination=policy[f"{alignment}_destination"])
                proof = "fire-return" if args.cause == "fire" else "death-return"
                result = subprocess.run(["node", f"web/proof/{proof}-proof.mjs"], cwd=REPOSITORY_ROOT,
                                        input=json.dumps(config), text=True, capture_output=True, timeout=180,
                                        env={**os.environ, "NODE_EXTRA_CA_CERTS": str(server.authority)})
                if result.returncode:
                    raise RuntimeError(f"{engine}/{alignment}: {result.stderr[-3500:]}")
                report = json.loads((output / f"{engine}-{alignment}.json").read_text())
                if args.cause == "ordinary":
                    report["restart"] = prove_restart_durability(server)
                reports.append(report)
                print(f"PASS {engine}/{alignment}", flush=True)
    receipt.write_text(json.dumps({"verdict": "INSPECTION" if args.engine else "PASS",
                                  "release": str(release), "fixture": fixture, "reports": reports}, indent=2) + "\n")


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
    document["world"].pop("character_presence", None)
    return digest, json.dumps(document, sort_keys=True)


def prove_restart_durability(server) -> dict:
    """Stop the serving process, serve the same database, and play again.

    The durable payload must be identical across the restart, and a fresh
    authenticated session must still select the same character, observe the same
    life state and location, and have an ordinary command accepted.
    """
    def observed(frame: dict) -> dict:
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
        }

    with LiveWireClient(server) as before:
        death = observed(before.frame)
    digest_before, payload_before = durable_checkpoint(server.database_url)
    server.restart()
    digest_after, payload_after = durable_checkpoint(server.database_url)
    if payload_before != payload_after:
        raise RuntimeError("the durable checkpoint payload changed across a serving-process restart")
    with LiveWireClient(server) as after:
        recovered = observed(after.frame)
        for field in ["life_state", "hp", "location", "actor_id", "character_id",
                      "carried_gold", "items", "skill_ledger"]:
            if recovered[field] != death[field]:
                raise RuntimeError(
                    f"restart changed {field}: {recovered[field]!r} != {death[field]!r}"
                )
        intent = {"kind": "request_resurrection"} if recovered["life_state"] == "ghost" else {"kind": "wait"}
        result, _ = after.command(intent)
        disposition = result.get("disposition", {})
        if disposition.get("kind") != "accepted":
            raise RuntimeError(f"a fresh authenticated command was refused after restart: {result!r}")
    return {"checkpoint_sha256_before": digest_before, "checkpoint_sha256_after": digest_after,
            "payload_unchanged_across_restart": True, "excluded_live_field": "character_presence",
            "before": death, "after": recovered, "accepted_intent": intent,
            "accepted_command": True}


if __name__ == "__main__":
    main()
