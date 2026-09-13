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
    # The presentation land gives the player defense 40 and its sole monster
    # attack 2; that opponent cannot hit an ordinary player. This explicit
    # disposable adversary fixture changes its attack, never the resolver. It
    # does not loot during the request delay, so the retained-inventory assertion
    # exercises resurrection rather than the independent corpse-scavenging path.
    adversary = next(actor for actor in catalog["actor_definitions"].values()
                     if actor["id"] == "actor/first_expedition/cellar_scavenger")
    if args.cause == "ordinary":
        adversary["stats"]["attack"] = 80
        adversary["scavenging_profile_id"] = None
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
               "fixture_catalog_sha256": hashlib.sha256(catalog_path.read_bytes()).hexdigest(),
               "cause": args.cause,
               "overrides": {"scavenger_attack": 80, "scavenging": False} if args.cause == "ordinary"
               else {"fire_proof_damage_kind": "fire", "potency": 100, "range": 0, "lane": "monster_special",
                     "cast_class": "not_applicable", "scavenger_ability": "fire_proof", "scavenging": False},
               "initial_player_hp": 1 if args.cause == "ordinary" else 40, "resource_maxima_unchanged": True}
    for engine in engines:
        for alignment, body in [("lawful", "male"), ("neutral", "female")]:
            world = replace(world_fixture(), catalog=str(catalog_path))
            player = next(actor for actor in world.generated_seed["actors"] if actor["id"] == world.controlled_actor)
            player["location"] = dict(realm="first_expedition", level="d1_entry", position=dict(x=23, y=9))
            player["character"]["identity"]["sex_or_gender_display"] = body
            player["character"]["alignment_state"]["alignment"] = alignment
            player["character"]["resources"]["hp"] = fixture["initial_player_hp"]
            monster = next(actor for actor in world.generated_seed["actors"] if actor["id"] == "cellar_scavenger")
            monster["location"] = copy.deepcopy(player["location"])
            # The production hold-ground monster cannot fight across squares.
            # A real East command joins it after the browser has observed life.
            monster["location"]["position"]["x"] = 24
            server = WorldServer(read_admin_url(args.admin_url_file), world, binary_path=release / "bin/tme-server")
            server.bundle, server.assets = release / "web", release / "web/feel-assets"
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
                reports.append(json.loads((output / f"{engine}-{alignment}.json").read_text()))
                print(f"PASS {engine}/{alignment}", flush=True)
    receipt.write_text(json.dumps({"verdict": "INSPECTION" if args.engine else "PASS",
                                  "release": str(release), "fixture": fixture, "reports": reports}, indent=2) + "\n")


if __name__ == "__main__":
    main()
