#!/usr/bin/env python3
"""Prove four-floor drawing and real traversal against an immutable private release."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess

from live_server_harness import REPOSITORY_ROOT, read_admin_url
from run_pixel_temple import PixelServer, temple_world


def location(level, x, y):
    return dict(realm="first_expedition", level=level, position=dict(x=x, y=y))


def cases():
    # Deliberately selected routes, not a claim of traversing every alternate stair.
    return [dict(scenario="doors", start=location("d1_entry", 23, 9)),
            dict(scenario="stairs-1", start=location("d1_entry", 15, 3), down=location("d2", 14, 3),
                 returnAt=dict(x=15, y=3), up=location("d1_entry", 16, 3)),
            dict(scenario="stairs-2", start=location("d2", 4, 3), down=location("d3", 1, 3),
                 returnAt=dict(x=2, y=3), up=location("d2", 5, 3)),
            dict(scenario="stairs-3", start=location("d3", 9, 17), down=location("d4", 7, 17),
                 returnAt=dict(x=8, y=17), up=location("d3", 10, 17)),
            dict(scenario="actor-actions", start=location("d4", 8, 17)),
            dict(scenario="martial-male", sex="male", start=location("d1_entry", 23, 9)),
            dict(scenario="martial-female", sex="female", start=location("d1_entry", 23, 9)),
            # The observed character defends. `Fight` is legal only on a shared
            # tile, and the authored monster holds ground, so the fixture colocates
            # them rather than asking the monster to chase.
            dict(scenario="defense-male", sex="male", defense=True, monster="cellar_scavenger",
                 start=location("d1_entry", 23, 9)),
            dict(scenario="defense-female", sex="female", defense=True, monster="cellar_scavenger",
                 start=location("d1_entry", 23, 9)),
            # Negative control: identical but for an occupied right hand, so no
            # martial hand candidate exists and the same attack cannot be blocked.
            dict(scenario="defense-occupied-hand", sex="male", defense=True, occupied=True,
                 monster="cellar_scavenger", start=location("d1_entry", 23, 9))]


def martial_fixture(seed, player, sex):
    """Colocated stationary targets allow a follow-up punch even after a fatal kick."""
    for actor_id, y in [("motion_target", 9), ("motion_fist", 9)]:
        target = copy.deepcopy(next(a for a in seed["actors"] if a["id"] == "lodge_keeper"))
        target.update(id=actor_id, location=location("d1_entry", 25, y))
        seed["actors"].append(target)
    player["character"]["identity"].update(base_class_id="martial_artist",
        current_class_id="martial_artist", display_class="Martial Artist", sex_or_gender_display=sex)
    player["character"]["skill_ledger"] = [s for s in player["character"]["skill_ledger"] if s["track_id"] == "hand"]
    next(s for s in player["character"]["skill_ledger"] if s["track_id"] == "hand")["level"] = 6
    # Retain the inventory while freeing the attacking hand.
    next(i for i in player["carried"]["items"] if i["position"] == "right_hand")["position"] = "sack_item_2"


def defense_fixture(seed, player, sex, occupied=False):
    """The observed character is the defender, not the attacker.

    A hold-ground monster never closes distance and `Fight` is legal only on a
    shared tile, so the authored `cellar_scavenger` is placed on the player's own
    tile. That exercises the existing automatic attack path without teaching the
    monster to chase, and without inventing a combat event.

    Hand level 19 against the unchanged authored curve gives the largest available
    hand-block threshold. Skill, equipment and the resolver curve are the only
    fixture inputs; the block itself is the production resolver's decision.
    """
    player["character"]["identity"].update(base_class_id="martial_artist",
        current_class_id="martial_artist", display_class="Martial Artist", sex_or_gender_display=sex)
    player["character"]["skill_ledger"] = [
        s for s in player["character"]["skill_ledger"] if s["track_id"] == "hand"]
    next(s for s in player["character"]["skill_ledger"] if s["track_id"] == "hand")["level"] = 19
    # The starting staff occupies the right hand. Freeing it is what makes the
    # martial hand candidate exist; the negative control deliberately leaves it.
    right_hand = next(i for i in player["carried"]["items"] if i["position"] == "right_hand")
    if occupied:
        assert right_hand["item_instance_id"] == "weathered_staff", (
            "the negative control must keep a real item in the right hand")
    else:
        right_hand["position"] = "sack_item_2"
    monster = next(a for a in seed["actors"] if a["id"] == "cellar_scavenger")
    monster["location"] = copy.deepcopy(player["location"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", required=True, type=Path)
    parser.add_argument("--admin-url-file", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--engine", choices=["chromium", "firefox", "webkit"])
    parser.add_argument("--scenario", choices=[case["scenario"] for case in cases()])
    args = parser.parse_args()
    release = args.release.resolve(strict=True)
    output = args.output.resolve()
    if release.is_relative_to(REPOSITORY_ROOT) or output.is_relative_to(REPOSITORY_ROOT):
        parser.error("release and proof output must be outside the checkout")
    receipt = json.loads((release / "release.json").read_text())
    actual = {str(p.relative_to(release)): hashlib.sha256(p.read_bytes()).hexdigest()
              for p in release.rglob("*") if p.is_file() and p != release / "release.json"}
    if any(p.is_symlink() for p in release.rglob("*")) or actual != receipt["files"]:
        parser.error("release differs from its integrity receipt")
    for name, digest in actual.items():
        if name.startswith("content/") and hashlib.sha256((REPOSITORY_ROOT / name).read_bytes()).hexdigest() != digest:
            parser.error("proof world content differs from the release")
    output.mkdir(parents=True, exist_ok=True)
    report = output / "verification.json"
    report.write_text(json.dumps(dict(verdict="INCOMPLETE")) + "\n")
    engines = [args.engine] if args.engine else json.loads((REPOSITORY_ROOT / "web/proof/engines.json").read_text())
    reports = []
    for engine in engines:
        for case in cases():
            if args.scenario and case["scenario"] != args.scenario:
                continue
            world = temple_world()
            player = next(a for a in world.generated_seed["actors"] if a["id"] == world.controlled_actor)
            player["location"] = case["start"]
            if case.get("defense"):
                defense_fixture(world.generated_seed, player, case["sex"],
                                occupied=bool(case.get("occupied")))
            elif "sex" in case:
                martial_fixture(world.generated_seed, player, case["sex"])
            server = PixelServer(read_admin_url(args.admin_url_file), world, binary_path=release / "bin/tme-server")
            server.bundle = release / "web"
            server.assets = release / "web/feel-assets"
            with server:
                config = dict(**case, engine=engine, origin=server.origin, authority=str(server.authority),
                              username=server.username, password=server.password, output=str(output))
                script = ("dungeon-block-proof.mjs" if case.get("defense") else
                          "dungeon-motion-proof.mjs" if "sex" in case else
                          "dungeon-actions-proof.mjs" if case["scenario"] == "actor-actions" else "dungeon-proof.mjs")
                result = subprocess.run(["node", str(REPOSITORY_ROOT / "web/proof" / script)],
                                        input=json.dumps(config), text=True, capture_output=True,
                                        env={**os.environ, "NODE_EXTRA_CA_CERTS": str(server.authority)})
                print(result.stdout, flush=True)
                if result.returncode:
                    raise RuntimeError(result.stderr[-3500:])
                reports.append(json.loads((output / f'{engine}-{case["scenario"]}.json').read_text()))
    report.write_text(json.dumps(dict(verdict="PASS", release=str(release), reports=reports), indent=2) + "\n")


if __name__ == "__main__":
    main()
