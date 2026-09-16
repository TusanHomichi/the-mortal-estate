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
from typing import Callable, Sequence

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
    seed_source_bytes = (release / "content/lands/first-expedition/simulation_seed.json").read_bytes()
    template_bytes = (release / "content/lands/first-expedition/generated/world_template.json").read_bytes()
    adversary = next(actor for actor in catalog["actor_definitions"].values()
                     if actor["id"] == "actor/first_expedition/cellar_scavenger")
    # This runner serves the release catalog unchanged, but its **seed is a
    # fixture**: the controlled character is placed at the encounter, given a
    # body, an alignment and a starting pool, and the opponent is placed beside
    # it. It also selects a preseeded character rather than creating one through
    # the ordinary creation flow, which is why the composed journey in
    # `tools/run_first_expedition_proof.py --journey death` exists and why this
    # receipt says exactly which kind of run it is.
    #
    # The immediate-fire route additionally needs an adversary fixture, because
    # production content authors no monster-dealt fire attack. That override is
    # recorded separately below and never stands in for ordinary gameplay.
    overrides = {}
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
        catalog_path = output / "combat-catalog.json"
        catalog_path.write_text(json.dumps(catalog) + "\n")
        overrides = {"fire_proof_damage_kind": "fire", "potency": 100, "range": 0,
                     "lane": "monster_special", "cast_class": "not_applicable",
                     "scavenger_ability": "fire_proof", "scavenging": False}
    fixture = {
        "kind": "modified_seed_with_release_catalog",
        "note": ("the release catalog is served unchanged; the seed is a fixture and the "
                 "character is preseeded, not created through the ordinary flow"),
        "cause": args.cause,
        "release": str(release),
        "catalog": {"path": str(catalog_path.relative_to(release)) if str(catalog_path).startswith(str(release)) else str(catalog_path),
                    "release_sha256": hashlib.sha256(catalog_source).hexdigest(),
                    "served_sha256": hashlib.sha256(catalog_path.read_bytes()).hexdigest(),
                    "catalog_changed": catalog_path.read_bytes() != catalog_source},
        "template_release_sha256": hashlib.sha256(template_bytes).hexdigest(),
        "template_served_sha256": hashlib.sha256(template_bytes).hexdigest(),
        "seed_release_sha256": hashlib.sha256(seed_source_bytes).hexdigest(),
        "seed_served_sha256": None,
        "seed_overrides": ["controlled_actor_location", "sex_or_gender_display", "alignment",
                           "current_hp", "opponent_location", "opponent_location_offset"],
        "catalog_overrides": overrides,
        "initial_player_hp": 40,
        "resource_maxima_unchanged": True,
        "character_created_through_ui": False,
    }
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
            if fixture["seed_served_sha256"] is None:
                # The seed the run actually serves, hashed after every fixture
                # change, so `seed_overrides` above cannot drift from reality.
                fixture["seed_served_sha256"] = hashlib.sha256(
                    json.dumps(world.generated_seed, sort_keys=True).encode("utf-8")
                ).hexdigest()
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
                # `journey` is the browser half's own name for this route, and it
                # is half of every handshake file name. Both halves build those
                # names from the same two fields, so a rename here without one
                # there leaves the child waiting for a file nobody writes.
                journey = "death" if args.cause == "ordinary" else "fire"
                config = dict(engine=engine, alignment=alignment, body=body, origin=server.origin,
                              authority=str(server.authority), username=server.username, password=server.password,
                              output=str(output), journey=journey, source=source_identity(),
                              destination=policy[f"{alignment}_destination"])
                proof = "fire-return" if args.cause == "fire" else "death-return"
                stem = f"{engine}-{journey}"
                request_path = output / f"{stem}-restart-request.json"
                complete_path = output / f"{stem}-restart-complete.json"
                ledger_path = output / f"{stem}-ledger-request.json"
                ledger_complete_path = output / f"{stem}-ledger-complete.json"
                ledger = LedgerAudit(server.database_url)
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
                    # The item and coin ledger is read from the stored checkpoint
                    # on the browser's behalf: the proof asserts against the
                    # authoritative world instead of against its own frames.
                    extra_handshakes=[
                        ProofHandshake("ledger", ledger_path, ledger_complete_path, ledger.answer)
                    ],
                    timeout=900,
                    environment={**os.environ, "NODE_EXTRA_CA_CERTS": str(server.authority)},
                )
                if result.returncode:
                    raise RuntimeError(f"{engine}/{alignment}: {result.stderr[-3500:]}")
                report = json.loads((output / f"{engine}-{alignment}.json").read_text())
                reports.append(report)
                print(f"PASS {engine}/{alignment}", flush=True)
    receipt.write_text(json.dumps({"verdict": "INSPECTION" if args.engine else "PASS",
                                  "source": source_identity(), "release": str(release),
                                  "fixture": fixture, "reports": reports}, indent=2) + "\n")


@dataclass
class ProofHandshake:
    """One file-pair conversation the browser proof can start with its runner.

    The child writes `request_path` when it needs something only the process
    owner can do; the runner calls `answer` with the parsed request and writes
    what it returns to `complete_path`. `kind` is for diagnostics only.
    """

    kind: str
    request_path: Path
    complete_path: Path
    answer: "Callable[[dict], dict]"
    answered_sequence: int = 0

    def clear(self) -> None:
        for path in (self.request_path, self.complete_path):
            path.unlink(missing_ok=True)

    def serve(self) -> dict | None:
        """Answer the next request on this file pair, exactly once each.

        The browser reuses the pair and numbers its requests, because a file that
        stays on disk cannot say whether it is the request just written or the
        one answered a moment ago. A handshake with a single request (the
        restart) carries sequence 1 and is answered once; the ledger's capture
        and audit are two requests and get two answers.
        """
        if not self.request_path.exists():
            return None
        requested = json.loads(self.request_path.read_text())
        sequence = int(requested.pop("sequence", 1))
        if sequence <= self.answered_sequence:
            return None
        self.answered_sequence = sequence
        response = self.answer(requested)
        response["sequence"] = sequence
        self.complete_path.write_text(json.dumps(response, indent=2) + "\n")
        return response


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
    extra_handshakes: "Sequence[ProofHandshake]" = (),
    timeout: float = 900.0,
    environment: dict | None = None,
    cwd: Path | None = None,
) -> ProofChildResult:
    """Run one browser proof process and collect it exactly once.

    The child reads its whole configuration from standard input before it does
    anything else, so the pipe is written and closed first: waiting for a
    restart request while stdin stayed open would deadlock both halves.

    Besides the restart it may ask for, the child can start any of
    `extra_handshakes` — the ledger audit is the other one — and each is answered
    exactly once while the child runs.

    Output is drained by a reader thread rather than by `communicate`, because
    `communicate` re-enters the closed stdin stream on some interpreters and a
    full pipe buffer would otherwise stall the child while this process waits on
    a file. A timeout or any other failure still terminates, drains and reaps the
    child, and re-raises the original error rather than a cleanup error.
    """
    handshakes = list(extra_handshakes)
    if on_restart_request is not None:
        handshakes.append(
            ProofHandshake("restart", request_path, complete_path, on_restart_request)
        )
    for handshake in handshakes:
        handshake.clear()
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
            for handshake in handshakes:
                response = handshake.serve()
                if handshake.kind == "restart" and response is not None:
                    restart = response
            if time.monotonic() > deadline:
                raise ProofChildFailure(f"the proof process exceeded {timeout:g}s")
            time.sleep(0.05)
        # One last look: a child can write its request and exit before the poll
        # above notices, and an unanswered request would otherwise be reported as
        # a missing file in the browser half rather than as the runner's miss.
        for handshake in handshakes:
            response = handshake.serve()
            if handshake.kind == "restart" and response is not None:
                restart = response
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


def source_identity() -> dict:
    """The checkout the proof code itself is running from.

    The release receipt binds the served binary and content; this binds the
    proof that drove it, so a receipt can say which source produced the run
    rather than leaving that to the PR description.
    """
    def git(*arguments: str) -> str:
        return subprocess.run(
            ["git", *arguments], cwd=REPOSITORY_ROOT, capture_output=True, text=True,
            check=False).stdout.strip()

    return {"head": git("rev-parse", "HEAD"), "tree": git("write-tree"),
            "worktree_dirty": bool(git("status", "--porcelain"))}


def stored_checkpoint(database_url: str) -> dict:
    """The stored checkpoint document, read only."""
    result = subprocess.run(
        ["psql", database_url, "-tA", "-v", "ON_ERROR_STOP=1", "-c",
         "SELECT encode(checkpoint_bytes,'escape') FROM tme.facets"],
        capture_output=True, text=True, check=False)
    if result.returncode:
        raise RuntimeError(f"checkpoint unavailable: {result.stderr.strip()}")
    return json.loads(result.stdout.strip())


def durable_checkpoint(database_url: str) -> tuple[str, str, dict]:
    """The stored checkpoint's digest, its gameplay payload, and its volatile marks.

    `character_presence` is the one field a restart legitimately moves: it
    records who is connected and when a session went absent, and the proof's own
    reconnect rewrites it. It is returned separately so the caller can judge the
    movement it permits rather than ignore it; every other field is compared
    verbatim, so a restart that changed location, life state, inventory,
    balances, progression, timers, topology or scheduling still fails.
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
    return digest, json.dumps(document, sort_keys=True), volatile


def checkpoint_ledger(document: dict) -> dict:
    """Every item instance and every coin the stored checkpoint holds.

    This is a read-only audit of the authoritative record, not a second
    inventory model: it resolves each instance to the collections the checkpoint
    actually has (ground, carried, corpse, merchant, locker, offer) and fails
    closed on an instance that is in none of them or in more than one. The typed
    rules owner for the same boundary is
    `crates/tme-rules/tests/first_expedition_death_ledger.rs`, which resolves
    `ItemLocation` inside the engine; this reads the durable copy of the same
    facts so a browser journey can assert against the world rather than against
    its own frames.
    """
    world = document["world"]
    locations: dict[str, list[str]] = {}

    def place(item_instance_id, label):
        locations.setdefault(item_instance_id, []).append(label)

    for ground in world.get("ground_items") or []:
        place(ground["item_instance_id"], f"ground:{position_label(ground['location'])}")
    for actor in world.get("actors") or []:
        holder = actor.get("character_id") or actor["id"]
        for position, item_instance_id in (actor.get("carried", {}).get("items") or {}).items():
            place(item_instance_id, f"carried:{holder}:{position}")
    for corpse_id, corpse in (world.get("corpses") or {}).items():
        for position, item_instance_id in (corpse.get("contents") or {}).items():
            place(item_instance_id, f"corpse:{corpse_id}:{position}")
    for inventory in world.get("merchant_inventories") or []:
        service = inventory.get("service_instance_id", "?")
        capability = inventory.get("capability_id", "?")
        for listing in inventory.get("listings") or []:
            place(listing["item_instance_id"], f"merchant:{service}:{capability}")
    for vault_id, vault in (world.get("locker_vaults") or {}).items():
        for owner, contents in (vault.get("lockers") or {}).items():
            for item_instance_id in contents:
                place(item_instance_id, f"locker:{vault_id}:{owner}")
    for item_instance_id, offer in (world.get("item_offers") or {}).items():
        place(
            item_instance_id,
            f"offered:{offer['sender_character_id']}:{offer['recipient_character_id']}",
        )

    items = {}
    for item_instance_id, instance in (world.get("item_instances") or {}).items():
        found = sorted(locations.get(item_instance_id, []))
        items[item_instance_id] = {
            "definition_id": instance["definition_id"],
            "quantity": instance["quantity"],
            "locations": found,
        }

    gold: dict[str, int] = {}
    for actor in world.get("actors") or []:
        holder = actor.get("character_id") or actor["id"]
        carried = (actor.get("carried") or {}).get("gold") or {}
        for position in ("left_hand", "right_hand", "sack"):
            amount = carried.get(position, 0)
            if amount:
                gold[f"actor:{holder}:{position}"] = amount
    for corpse_id, corpse in (world.get("corpses") or {}).items():
        if corpse.get("gold"):
            gold[f"corpse:{corpse_id}"] = corpse["gold"]
    for pile_id, pile in (world.get("ground_gold") or {}).items():
        if pile.get("amount"):
            gold[f"ground:{pile_id}"] = pile["amount"]
    for bank_id, bank in (world.get("banks") or {}).items():
        for character_id, balance in (bank.get("balances") or {}).items():
            if balance:
                gold[f"bank:{bank_id}:{character_id}"] = balance

    return {
        "items": items,
        "gold": gold,
        "item_count": len(items),
        "gold_total": sum(gold.values()),
        "unowned": sorted(
            item_instance_id
            for item_instance_id, row in items.items()
            if len(row["locations"]) != 1
        ),
    }


def position_label(position: dict) -> str:
    """One world position as a stable, comparable label."""
    coord = position["position"]
    return f"{position['realm']}/{position['level']}/{coord['x']},{coord['y']}"


def ledger_summary(ledger: dict) -> dict:
    """The compact, JSON-safe account of one ledger reading."""
    return {
        "item_count": ledger["item_count"],
        "gold_total": ledger["gold_total"],
        "unowned": ledger["unowned"],
        "ledger_sha256": hashlib.sha256(
            json.dumps(ledger, sort_keys=True).encode("utf-8")
        ).hexdigest(),
    }


def ledger_movement(before: dict, after: dict) -> dict:
    """Where every instance went, so a receipt can name the legitimate thefts."""
    movement = {}
    for item_instance_id in sorted(set(before["items"]) | set(after["items"])):
        source = before["items"].get(item_instance_id, {}).get("locations")
        destination = after["items"].get(item_instance_id, {}).get("locations")
        if source != destination:
            movement[item_instance_id] = {"from": source, "to": destination}
    return movement


class LedgerAudit:
    """The runner's half of the browser's ledger handshake.

    The browser asks for a capture before the boundary it cares about and for an
    audit after it; the answer is computed here, from the stored checkpoint, so a
    browser journey asserts against the authoritative world rather than against
    its own frames. An audit whose capture is missing is refused rather than
    silently compared against an empty ledger.
    """

    def __init__(self, database_url: str):
        self.database_url = database_url
        self.captures: dict[str, dict] = {}

    def answer(self, requested: dict) -> dict:
        action = requested.get("action")
        label = requested.get("label")
        if action == "capture":
            ledger = checkpoint_ledger(stored_checkpoint(self.database_url))
            self.captures[label] = ledger
            return {"verdict": "captured", "label": label, **ledger_summary(ledger)}
        if action == "audit":
            if label not in self.captures:
                return {
                    "verdict": "refused",
                    "label": label,
                    "defects": [f"no ledger was captured under {label!r}"],
                }
            before = self.captures[label]
            after = checkpoint_ledger(stored_checkpoint(self.database_url))
            return {
                "verdict": "audited",
                "label": label,
                "defects": audit_ledgers(before, after),
                "before": ledger_summary(before),
                "after": ledger_summary(after),
                "movement": ledger_movement(before, after),
            }
        return {
            "verdict": "refused",
            "label": label,
            "defects": [f"unknown ledger action {action!r}"],
        }


def audit_ledgers(before: dict, after: dict) -> list[str]:
    """Every way a later ledger can fail to be the same ledger, named.

    The same rule the rules test owns, applied to two read-only readings of the
    stored checkpoint: instances may move but not stop existing, appear,
    duplicate, change identity or change quantity, and coins may move but not be
    created or destroyed anywhere in the world. Deliberate theft is a move, so it
    passes; anything the return invents fails.
    """
    defects = []
    lost = sorted(set(before["items"]) - set(after["items"]))
    if lost:
        defects.append(f"item instances stopped existing: {lost}")
    created = sorted(set(after["items"]) - set(before["items"]))
    if created:
        defects.append(f"item instances appeared from nowhere: {created}")
    for item_instance_id in sorted(set(before["items"]) & set(after["items"])):
        source, destination = before["items"][item_instance_id], after["items"][item_instance_id]
        if source["definition_id"] != destination["definition_id"]:
            defects.append(
                f"item instance {item_instance_id!r} changed identity: "
                f"{source['definition_id']} -> {destination['definition_id']}"
            )
        if source["quantity"] != destination["quantity"]:
            defects.append(
                f"item instance {item_instance_id!r} changed quantity: "
                f"{source['quantity']} -> {destination['quantity']}"
            )
    for item_instance_id in after["unowned"]:
        defects.append(
            f"item instance {item_instance_id!r} is not in exactly one place: "
            f"{after['items'][item_instance_id]['locations']}"
        )
    if before["gold_total"] != after["gold_total"]:
        defects.append(
            f"gold was created or destroyed: {before['gold_total']} -> {after['gold_total']}"
        )
    return defects


def observe_frame(frame: dict) -> dict:
    """The durable facts one observer frame states about its own character."""
    actor_id = frame["observer_actor_id"]
    self_row = next(row for row in frame["actors"] if row["actor_id"] == actor_id)
    # Carried rows nest the instance under `item`; ground rows and their siblings
    # flatten it. The same reader handles both, so a frame that changes shape
    # cannot quietly report an empty inventory.
    instance_of = lambda row: (row.get("item") or row)["item_instance_id"]
    return {
        "life_state": self_row["life_state"],
        "hp": self_row["hp"],
        "location": frame["observation_center"],
        "actor_id": actor_id,
        "character_id": frame["social"]["character_id"],
        "carried_gold": frame["carried"]["gold"]["sack"],
        "items": sorted(instance_of(row) for row in frame["carried"]["items"]),
        "skill_ledger": frame["character"]["skill_ledger"],
        "ready_at": frame["ready_at"],
        "logical_time": frame["logical_time"],
        "can_act": frame["can_act"],
    }


def judge_presence_movement(before: dict, after: dict) -> list[str]:
    """The only presence differences a restart may legitimately produce.

    Both arguments are the presence ledger alone: character ID to that
    character's own marks. `character_presence` records who is connected and
    when a session went absent, so a restart that drops the old session
    legitimately moves `connected` and `absent_since`. Everything else about it
    is gameplay state and is judged: a character appearing in or vanishing from
    the ledger, or a control epoch moving, is an unexplained difference rather
    than a lifecycle mark.
    """
    defects = []
    if set(before) != set(after):
        added = sorted(set(after) - set(before))
        removed = sorted(set(before) - set(after))
        defects.append(
            f"the restart changed which characters hold a presence mark: +{added} -{removed}"
        )
    for character_id in sorted(set(before) & set(after)):
        source, destination = before[character_id], after[character_id]
        if source.get("control_epoch") != destination.get("control_epoch"):
            defects.append(
                f"the restart moved character {character_id!r} control epoch "
                f"{source.get('control_epoch')!r} -> {destination.get('control_epoch')!r}"
            )
        unexplained = sorted(
            key
            for key in set(source) | set(destination)
            if key not in ("connected", "absent_since", "control_epoch")
            and source.get(key) != destination.get(key)
        )
        if unexplained:
            defects.append(
                f"the restart changed unexplained presence fields for {character_id!r}: {unexplained}"
            )
    return defects


def judge_volatile_fields(before: dict, after: dict) -> list[str]:
    """Judge the named lifecycle fields instead of ignoring them wholesale.

    Each entry in [`VOLATILE_CHECKPOINT_FIELDS`] has its own rule. A field with
    no rule yet is refused rather than excluded silently, and a field that
    appears outside the named set means the exclusion list has drifted from the
    checkpoint it describes.
    """
    defects = []
    for field in VOLATILE_CHECKPOINT_FIELDS:
        if field == "character_presence":
            defects.extend(judge_presence_movement(before.get(field) or {}, after.get(field) or {}))
        elif before.get(field) != after.get(field):
            defects.append(
                f"the restart changed the volatile field {field!r}, which has no judging rule"
            )
    unclassified = sorted((set(before) | set(after)) - set(VOLATILE_CHECKPOINT_FIELDS))
    if unclassified:
        defects.append(f"the restart reported unclassified volatile fields: {unclassified}")
    return defects


def compare_checkpoints(
    *,
    digest_before: str,
    digest_after: str,
    gameplay_before: str,
    gameplay_after: str,
    volatile_before: dict,
    volatile_after: dict,
) -> dict:
    """Judge what a restart did to the stored checkpoint, from what was read.

    Two claims live here and they are not the same claim. `checkpoint_bytes_identical`
    reports the raw stored digest, which a legitimate presence rewrite may move.
    `durable_payload_unchanged` compares every gameplay field verbatim — the
    presence ledger is the only field removed, and it is judged separately by
    [`judge_volatile_fields`] rather than ignored. Every field of this receipt is
    computed from its arguments, so a receipt cannot claim a comparison that was
    never run.
    """
    defects = judge_volatile_fields(volatile_before, volatile_after)
    unchanged = gameplay_before == gameplay_after
    if not unchanged:
        offset = next(
            (
                index
                for index, (left, right) in enumerate(zip(gameplay_before, gameplay_after))
                if left != right
            ),
            min(len(gameplay_before), len(gameplay_after)),
        )
        defects.append(
            "the durable checkpoint payload changed across a serving-process restart "
            f"at offset {offset}: "
            f"{gameplay_before[max(0, offset - 80):offset + 80]!r} != "
            f"{gameplay_after[max(0, offset - 80):offset + 80]!r}"
        )
    return {
        "checkpoint_sha256_before": digest_before,
        "checkpoint_sha256_after": digest_after,
        "checkpoint_bytes_identical": digest_before == digest_after,
        "payload_comparison": "parsed_json_minus_named_volatile_fields",
        "durable_payload_unchanged": unchanged,
        "excluded_volatile_fields": list(VOLATILE_CHECKPOINT_FIELDS),
        "volatile_before": volatile_before,
        "volatile_after": volatile_after,
        "volatile_presence_movement": volatile_before != volatile_after,
        "defects": defects,
    }


def judge_dead_actor_attempts(attempts: dict, *, logical_time, ready_at) -> dict:
    """Judge what a restarted server actually answered a ghost.

    `attempts` is the wire session's own record: for each intent it sent, the
    disposition, command id and server sequence that came back. Nothing is
    assumed. A ghost may not perform a physical or sheet action at any time, so
    one that was not refused is a defect. The return request is only ever sent
    while the character is still inside its own authored deadline — asking then
    can only be refused, and asking changes nothing — and once the deadline has
    elapsed the session must *not* ask, because the browser's own authorized
    return is what proves acceptance and consuming it here would take that proof
    away from the half that can observe it. Every receipt field below is computed
    from the observation it names.
    """
    defects = []
    eligibility = int(logical_time) >= int(ready_at)
    return_order = attempts.get("request_resurrection")
    physical_order = attempts.get("show_sack", {})
    physical_kind = physical_order.get("disposition", {}).get("kind")
    return_kind = (return_order or {}).get("disposition", {}).get("kind")
    if physical_kind != "rejected":
        defects.append(
            "a dead actor was not refused a physical or sheet action after a restart: "
            f"{physical_order!r}"
        )
    if not eligibility and return_order is None:
        defects.append(
            "the character was still inside its own deadline and its return request was never "
            "attempted, so the refusal was never observed"
        )
    if eligibility and return_order is not None:
        defects.append(
            "the session consumed the return of an eligible character, which the browser half's "
            f"own accepted return has to prove: {return_order!r}"
        )
    if return_order is not None:
        return_kind = return_order.get("disposition", {}).get("kind")
        if return_kind != "rejected":
            defects.append(
                "the restarted server did not refuse the return of a character still inside its "
                f"own deadline: {return_order!r}"
            )
    return {
        "command_attempts": attempts,
        "physical_action_refused_while_dead": physical_kind == "rejected",
        "return_attempted_while_ineligible": return_order is not None,
        "return_refused_before_deadline": return_kind == "rejected" and not eligibility,
        "return_left_to_the_browser": return_order is None and eligibility,
        "eligible_at_restart": eligibility,
        "eligibility_evaluated_at": {"logical_time": logical_time, "ready_at": ready_at},
        "defects": defects,
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
    # The browser half may be observing a character it created itself, which the
    # harness did not enroll. Selecting it by the ID the browser reported is what
    # makes this session's observations about the same character.
    character_id = (expect or {}).get("character_id")

    with LiveWireClient(server, character_id=character_id) as before:
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
    comparison = compare_checkpoints(
        digest_before=digest_before,
        digest_after=digest_after,
        gameplay_before=payload_before,
        gameplay_after=payload_after,
        volatile_before=volatile_before,
        volatile_after=volatile_after,
    )
    if comparison["defects"]:
        raise RuntimeError("; ".join(comparison["defects"]))
    with LiveWireClient(server, character_id=character_id) as after:
        recovered = observed(after.frame)
        for field in ["life_state", "hp", "location", "actor_id", "character_id",
                      "carried_gold", "items", "skill_ledger", "ready_at"]:
            if recovered[field] != death[field]:
                raise RuntimeError(
                    f"restart changed {field}: {recovered[field]!r} != {death[field]!r}"
                )
        # This is a fresh authenticated session against the restarted process.
        # What it answers is recorded, not assumed. The return request is asked
        # only while the character is still inside its own deadline, where the
        # only correct answer is a refusal; once the deadline has elapsed the
        # browser half's own accepted return is the proof, and asking here would
        # both consume it and take that observation away from the half that can
        # make it. The physical action is asked in both cases: a ghost may never
        # perform one, eligible or not.
        eligibility = int(recovered["logical_time"]) >= int(recovered["ready_at"])
        attempts = {}
        intents = [{"kind": "show_sack"}]
        if not eligibility:
            intents.insert(0, {"kind": "request_resurrection"})
        for intent in intents:
            result, _ = after.command(intent)
            attempts[intent["kind"]] = {
                "disposition": result.get("disposition", {}),
                "command_id": result.get("command_id"),
                "server_sequence": result.get("server_sequence"),
            }
        judgment = judge_dead_actor_attempts(
            attempts,
            logical_time=recovered["logical_time"],
            ready_at=recovered["ready_at"],
        )
        if judgment["defects"]:
            raise RuntimeError("; ".join(judgment["defects"]))
    return {
        **comparison,
        "before": death,
        "after": recovered,
        **{key: value for key, value in judgment.items() if key != "defects"},
    }


if __name__ == "__main__":
    main()
