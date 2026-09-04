#!/usr/bin/env python3
"""Record and verify the representative presentation-adoption observer frame.

The tracked fixture names only relocations of facts already carried by the
identity-proof simulation seed. This driver derives an ordinary simulation
seed, serves it through the real TLS server, and asks the shipped Godot client
to record the first authoritative frame satisfying the fixture's semantic
barrier. It never edits the standing seed.

The ordinary verification route writes run evidence below ``--output`` and
compares a normalized semantic projection with the tracked frame. Replacing the
tracked frame requires the explicit ``--record-frame`` option.
"""

from __future__ import annotations

import argparse
import copy
import dataclasses
import datetime as dt
import hashlib
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

REPOSITORY_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPOSITORY_ROOT / "tools"))

from live_server_harness import (  # noqa: E402
    GODOT_VERSION,
    LiveServer,
    ProofError,
    World,
    emit_client_output,
    read_admin_url,
    resolve_godot,
    run,
)

SUCCESS_SENTINEL = "TME_PRESENTATION_ADOPTION_RECORDING_OK"
CLIENT_SENTINEL = "TME_PRESENTATION_ADOPTION_FRAME_OK"
CLIENT_SCRIPT = "res://tests/record_presentation_adoption_frame.gd"
FIXTURE_PATH = Path("tests/fixtures/presentation-adoption/representative-recording.json")
TRACKED_FRAME_PATH = Path(
    "tests/fixtures/presentation-adoption/identity-proof-observer-frame.json"
)
FIXTURE_KIND = "presentation_adoption_recording_fixture"
FRAME_KIND = "capture_frame_fixture"


def canonical_json(document: Any) -> bytes:
    return (json.dumps(document, indent=2, sort_keys=True) + "\n").encode()


def sha256_bytes(payload: bytes) -> str:
    return hashlib.sha256(payload).hexdigest()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_value(*arguments: str) -> str:
    return subprocess.run(
        ["git", "-C", str(REPOSITORY_ROOT), *arguments],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def command_version(*arguments: str) -> str:
    return subprocess.run(
        list(arguments), check=True, capture_output=True, text=True
    ).stdout.strip()


def load_fixture(path: Path = REPOSITORY_ROOT / FIXTURE_PATH) -> dict[str, Any]:
    document = json.loads(path.read_text(encoding="utf-8"))
    expected_keys = {
        "schema_version",
        "kind",
        "source",
        "arrangements",
        "capture_barrier",
        "evidence_limits",
    }
    if set(document) != expected_keys:
        raise ProofError(
            f"{path.relative_to(REPOSITORY_ROOT)} fields are {sorted(document)}; "
            f"expected exactly {sorted(expected_keys)}"
        )
    if document["schema_version"] != 1 or document["kind"] != FIXTURE_KIND:
        raise ProofError(f"{path.relative_to(REPOSITORY_ROOT)} is not a version 1 {FIXTURE_KIND}")
    if document["evidence_limits"] != {
        "dead_layer": False,
        "static_prop_required": False,
        "transition_aperture_required": False,
    }:
        raise ProofError("the representative fixture must preserve its explicit evidence limits")
    arrangements = document["arrangements"]
    if set(arrangements) != {"actors", "ecology_members", "services"}:
        raise ProofError("arrangements may name actors, ecology_members, and services only")
    if any(len(arrangements[name]) != 1 for name in ("actors", "ecology_members", "services")):
        raise ProofError("the fixture must carry exactly one actor, ecology-member, and service relocation")
    return document


def _replacement_location(prior: dict[str, Any], replacement: dict[str, Any]) -> dict[str, Any]:
    if set(replacement) != {"level", "position", "realm"}:
        raise ProofError("a fixture location may contain level, position, and realm only")
    if set(replacement["position"]) != {"x", "y"}:
        raise ProofError("a fixture position may contain x and y only")
    if replacement["level"] != prior["level"] or replacement["realm"] != prior["realm"]:
        raise ProofError("the fixture may relocate a fact inside its level and realm, not move it between them")
    if not all(isinstance(replacement["position"][axis], int) for axis in ("x", "y")):
        raise ProofError("fixture coordinates must be integers")
    return copy.deepcopy(replacement)


def arrange_seed(
    source_seed: dict[str, Any], fixture: dict[str, Any]
) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    """Apply only the three fixture-owned relocations to a deep copy."""
    effective = copy.deepcopy(source_seed)
    deltas: list[dict[str, Any]] = []

    actor_change = fixture["arrangements"]["actors"][0]
    actor_rows = [row for row in effective["actors"] if row.get("id") == actor_change["actor_id"]]
    if len(actor_rows) != 1:
        raise ProofError(f"source seed must carry actor {actor_change['actor_id']!r} exactly once")
    actor = actor_rows[0]
    actor_before = copy.deepcopy(actor["location"])
    actor["location"] = _replacement_location(actor_before, actor_change["location"])
    deltas.append(
        {
            "path": f"actors/{actor_change['actor_id']}/location",
            "before": actor_before,
            "after": copy.deepcopy(actor["location"]),
        }
    )

    ecology_change = fixture["arrangements"]["ecology_members"][0]
    sites = [
        row for row in effective["ecology_sites"] if row.get("id") == ecology_change["site_id"]
    ]
    if len(sites) != 1:
        raise ProofError(f"source seed must carry ecology site {ecology_change['site_id']!r} exactly once")
    locations = sites[0].get("member_locations", {})
    member_id = ecology_change["member_id"]
    if member_id not in locations:
        raise ProofError(f"ecology site {ecology_change['site_id']!r} has no member {member_id!r}")
    ecology_before = copy.deepcopy(locations[member_id])
    locations[member_id] = _replacement_location(ecology_before, ecology_change["location"])
    deltas.append(
        {
            "path": f"ecology_sites/{ecology_change['site_id']}/member_locations/{member_id}",
            "before": ecology_before,
            "after": copy.deepcopy(locations[member_id]),
        }
    )

    service_change = fixture["arrangements"]["services"][0]
    service_rows = [
        row
        for row in effective["service_instances"]
        if row.get("id") == service_change["service_id"]
    ]
    if len(service_rows) != 1:
        raise ProofError(f"source seed must carry service {service_change['service_id']!r} exactly once")
    service = service_rows[0]
    service_before = copy.deepcopy(service["location"])
    service["location"] = _replacement_location(service_before, service_change["location"])
    deltas.append(
        {
            "path": f"service_instances/{service_change['service_id']}/location",
            "before": service_before,
            "after": copy.deepcopy(service["location"]),
        }
    )
    return effective, deltas


def _find_one(rows: list[dict[str, Any]], key: str, value: str, label: str) -> dict[str, Any]:
    matches = [row for row in rows if row.get(key) == value]
    if len(matches) != 1:
        raise ProofError(f"observed frame must carry {label} {value!r} exactly once")
    return matches[0]


def _actor_projection(actor: dict[str, Any]) -> dict[str, Any]:
    return {
        key: copy.deepcopy(actor.get(key))
        for key in (
            "actor_id",
            "attack_safety",
            "hp",
            "kind",
            "life_state",
            "max_hp",
            "name",
            "position",
        )
    }


def normalized_projection(document: dict[str, Any]) -> dict[str, Any]:
    """Fields compared across runs; run identities and timing are excluded."""
    frame = document["frame"]
    return {
        "observer_actor_id": frame.get("observer_actor_id"),
        "observation_center": copy.deepcopy(frame.get("observation_center")),
        "observation_radius": frame.get("observation_radius"),
        "actors": sorted(
            (_actor_projection(row) for row in frame.get("actors", [])),
            key=lambda row: str(row["actor_id"]),
        ),
        "action_options": copy.deepcopy(frame.get("action_options", [])),
        "action_options_truncated": frame.get("action_options_truncated"),
        "npcs_here": copy.deepcopy(frame.get("npcs_here", [])),
        "services_here": copy.deepcopy(frame.get("services_here", [])),
        "static_scene_context": copy.deepcopy(frame.get("static_scene_context")),
        "tiles": copy.deepcopy(frame.get("tiles", [])),
    }


def validate_frame(document: dict[str, Any], fixture: dict[str, Any]) -> dict[str, Any]:
    if document.get("schema_version") != 1 or document.get("kind") != FRAME_KIND:
        raise ProofError(f"recorded output is not a version 1 {FRAME_KIND}")
    frame = document.get("frame")
    if not isinstance(frame, dict):
        raise ProofError("recorded output carries no frame")
    barrier = fixture["capture_barrier"]
    if int(frame.get("logical_time", -1)) < barrier["minimum_logical_time"]:
        raise ProofError("recorded frame precedes the semantic capture barrier")
    if frame.get("observer_actor_id") != barrier["observer_actor_id"]:
        raise ProofError("recorded frame belongs to the wrong observer")
    actors = frame.get("actors", [])
    if len(actors) != barrier["actor_count"]:
        raise ProofError(f"recorded frame carries {len(actors)} actors, expected {barrier['actor_count']}")
    for expected in barrier["actors"]:
        actual = _find_one(actors, "actor_id", expected["actor_id"], "actor")
        for field in ("attack_safety", "kind", "life_state", "position"):
            if actual.get(field) != expected[field]:
                raise ProofError(
                    f"actor {expected['actor_id']!r} has {field}={actual.get(field)!r}; "
                    f"expected {expected[field]!r}"
                )

    expected_action = barrier["action_option"]
    action = _find_one(frame.get("action_options", []), "id", expected_action["id"], "action")
    for field in ("blocked_reason", "enabled", "label"):
        if action.get(field) != expected_action[field]:
            raise ProofError(f"representative action has unexpected {field}")

    expected_npc = barrier["npc_interaction"]
    npc = _find_one(frame.get("npcs_here", []), "actor_id", expected_npc["actor_id"], "NPC")
    interactions = npc.get("interactions", [])
    _find_one(interactions, "interaction_id", expected_npc["interaction_id"], "interaction")

    expected_service = barrier["service"]
    service = _find_one(
        frame.get("services_here", []), "service_id", expected_service["service_id"], "service"
    )
    if service.get("position") != expected_service["position"]:
        raise ProofError("representative service is not co-located with its keeper")
    capabilities = service.get("capabilities", [])
    _find_one(
        capabilities,
        "capability_id",
        expected_service["capability_id"],
        "service capability",
    )

    context = frame.get("static_scene_context", {})
    terrain_ids = {
        terrain
        for tile in context.get("tiles", [])
        for terrain in tile.get("terrain_ids", [])
    }
    missing = set(barrier["required_terrain_ids"]) - terrain_ids
    if missing:
        raise ProofError(f"recorded static scene lacks required terrain ids: {sorted(missing)}")
    return normalized_projection(document)


def proof(arguments: argparse.Namespace) -> int:
    godot = resolve_godot(arguments.godot)
    fixture_path = (REPOSITORY_ROOT / FIXTURE_PATH).resolve()
    fixture = load_fixture(fixture_path)
    source = fixture["source"]
    declared = World.declared(source["world_document"], key="presentation-adoption-recording")
    if declared.simulation_seed != source["simulation_seed"]:
        raise ProofError("the recording fixture no longer names the seed declared by its world")
    source_seed_path = REPOSITORY_ROOT / source["simulation_seed"]
    if sha256(source_seed_path) != source["simulation_seed_sha256"]:
        raise ProofError("the standing simulation seed drifted from the fixture's exact source digest")
    source_seed = json.loads(source_seed_path.read_text(encoding="utf-8"))
    effective_seed, deltas = arrange_seed(source_seed, fixture)

    output = Path(arguments.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    effective_seed_path = output / "effective-simulation-seed.json"
    effective_seed_path.write_bytes(canonical_json(effective_seed))
    frame_path = output / TRACKED_FRAME_PATH.name

    source_commit = git_value("rev-parse", "HEAD")
    source_tree = git_value("rev-parse", "HEAD^{tree}")
    tracked_status_before = git_value("status", "--short", "--untracked-files=no").splitlines()
    runtime_paths = git_value("ls-files", "--cached", "--others", "--exclude-standard", "--", "crates", "client", "tools", "Cargo.toml", "Cargo.lock").splitlines()
    working_sources = {path: sha256(REPOSITORY_ROOT / path) for path in sorted(set(runtime_paths)) if (REPOSITORY_ROOT / path).is_file()}
    working_source_sha256 = sha256_bytes(canonical_json(working_sources))
    world = dataclasses.replace(declared, simulation_seed=None, generated_seed=effective_seed)
    client_script = REPOSITORY_ROOT / "client/tests/record_presentation_adoption_frame.gd"
    started = time.monotonic()
    previous_cwd = Path.cwd()
    os.chdir(REPOSITORY_ROOT)
    try:
        with LiveServer(
            read_admin_url(arguments.admin_url_file), world, keep=arguments.keep
        ) as server:
            client = server.run_client(
                godot,
                CLIENT_SCRIPT,
                extra_environment={
                    "TME_PRESENTATION_FRAME_OUT": str(frame_path),
                    "TME_PRESENTATION_FIXTURE": str(fixture_path),
                    "TME_PRESENTATION_SOURCE_COMMIT": source_commit,
                    "TME_PRESENTATION_SOURCE_TREE": source_tree,
                },
                timeout=arguments.timeout,
            )
            emit_client_output(client)
            if client.returncode != 0 or CLIENT_SENTINEL not in client.stdout:
                raise ProofError(
                    f"presentation-adoption recorder failed with status {client.returncode}"
                )
            server_status = dict(server.status)
    finally:
        os.chdir(previous_cwd)

    document = json.loads(frame_path.read_text(encoding="utf-8"))
    projection = validate_frame(document, fixture)
    projection_digest = sha256_bytes(canonical_json(projection))
    expected_path = (
        Path(arguments.expected_frame).resolve()
        if arguments.expected_frame
        else (REPOSITORY_ROOT / TRACKED_FRAME_PATH)
    )
    expected_comparison: dict[str, Any] = {"performed": False, "equal": None}
    if not arguments.record_frame and expected_path.is_file() and expected_path != frame_path:
        expected_document = json.loads(expected_path.read_text(encoding="utf-8"))
        expected_projection = validate_frame(expected_document, fixture)
        expected_digest = sha256_bytes(canonical_json(expected_projection))
        expected_comparison = {
            "performed": True,
            "equal": expected_projection == projection,
            "expected_frame": str(expected_path.relative_to(REPOSITORY_ROOT)),
            "expected_frame_sha256": sha256(expected_path),
            "expected_normalized_projection_sha256": expected_digest,
        }
        if expected_projection != projection:
            raise ProofError(
                "the rerun differs from the tracked frame over the normalized semantic projection: "
                f"expected {expected_digest}, observed {projection_digest}"
            )

    server_binary = REPOSITORY_ROOT / "target/debug/tme-server"
    receipt = {
        "schema_version": 1,
        "kind": "presentation_adoption_recording_receipt",
        "disposition": "prerequisite_candidate_pending_non_author_rerun",
        "recorded_at_utc": dt.datetime.now(dt.UTC).isoformat(),
        "source": {
            "commit": source_commit,
            "tree": source_tree,
            "tracked_status_before": tracked_status_before,
            "working_source_sha256": working_source_sha256,
            "working_source_file_count": len(working_sources),
            "world_document": source["world_document"],
            "world_document_sha256": sha256(REPOSITORY_ROOT / source["world_document"]),
            "world_template": declared.world_template,
            "world_template_sha256": sha256(REPOSITORY_ROOT / declared.world_template),
            "simulation_seed": source["simulation_seed"],
            "simulation_seed_sha256": sha256(source_seed_path),
            "recording_fixture": str(FIXTURE_PATH),
            "recording_fixture_sha256": sha256(fixture_path),
        },
        "effective_input": {
            "simulation_seed_sha256": sha256(effective_seed_path),
            "arrangement_deltas": deltas,
        },
        "observed": {
            "frame_sha256": sha256(frame_path),
            "normalized_projection_sha256": projection_digest,
            "actor_count": len(document["frame"]["actors"]),
            "logical_time": document["frame"]["logical_time"],
            "observation_center": document["frame"]["observation_center"],
            "server_status": server_status,
        },
        "rerun_policy": {
            "compare": "normalized_projection_sha256",
            "compared_fields": list(projection),
            "excluded_run_local_fields": [
                "actors[].character_id",
                "frame_generation",
                "provenance.server_sequence",
                "provenance.world_revision",
                "receipt paths",
                "ports",
                "process ids",
                "timing",
            ],
            "expected_comparison": expected_comparison,
        },
        "toolchain": {
            "host": platform.platform(),
            "python": platform.python_version(),
            "godot_version": run([str(godot), "--version"]).strip(),
            "godot_binary_sha256": sha256(godot),
            "postgres_client": command_version("psql", "--version"),
            "openssl": command_version("openssl", "version"),
            "server_binary_sha256": sha256(server_binary),
        },
        "proof_sources": {
            "driver_sha256": sha256(Path(__file__).resolve()),
            "client_recorder_sha256": sha256(client_script),
        },
        "canonical_command": [
            "python3",
            "tools/run_presentation_adoption_recording.py",
            "--admin-url-file",
            "$TME_PG_ADMIN_URL_FILE",
            "--godot",
            "$TME_GODOT",
            "--output",
            "$TME_CAPTURE_OUTPUT/presentation-adoption",
            "--expected-frame",
            str(TRACKED_FRAME_PATH),
        ],
        "elapsed_seconds": round(time.monotonic() - started, 3),
        "evidence_limits": fixture["evidence_limits"],
    }
    receipt_path = output / "identity-proof-observer-frame.receipt.json"
    receipt_path.write_bytes(canonical_json(receipt))

    if arguments.record_frame:
        destination = Path(arguments.record_frame).resolve()
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(frame_path.read_bytes())
        destination.with_suffix(".receipt.json").write_bytes(receipt_path.read_bytes())
        print(f"recorded frame: {destination}")
    print(json.dumps(receipt, indent=2, sort_keys=True))
    print(SUCCESS_SENTINEL)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--admin-url-file", required=True)
    parser.add_argument("--godot", default=os.environ.get("TME_GODOT", ""))
    parser.add_argument("--output", required=True)
    parser.add_argument("--expected-frame")
    parser.add_argument("--record-frame")
    parser.add_argument("--timeout", type=float, default=300.0)
    parser.add_argument("--keep", action="store_true")
    arguments = parser.parse_args()
    if not arguments.godot:
        parser.error(f"--godot or TME_GODOT must name pinned Godot {GODOT_VERSION}")
    try:
        return proof(arguments)
    except (OSError, ValueError, json.JSONDecodeError, ProofError) as error:
        print(f"presentation-adoption recording failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
