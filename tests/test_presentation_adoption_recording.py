"""Proof-harness tests for the representative observer-frame recording."""

from __future__ import annotations

import copy
import json
import re
import unittest

import boundary_test_support  # noqa: F401  (puts tools/ on sys.path)
import run_presentation_adoption_recording as recording
from live_server_harness import ProofError, REPOSITORY_ROOT, World


def synthetic_frame(fixture: dict) -> dict:
    barrier = fixture["capture_barrier"]
    actors = []
    for expected in barrier["actors"]:
        actors.append(
            {
                **copy.deepcopy(expected),
                "character_id": "run-local-character" if expected["kind"] == "player" else None,
                "hp": 20,
                "max_hp": 20,
                "name": expected["actor_id"],
            }
        )
    action = {**copy.deepcopy(barrier["action_option"]), "intent": {"kind": "interact"}}
    interaction = {
        "interaction_id": barrier["npc_interaction"]["interaction_id"],
        "actions": [copy.deepcopy(action)],
    }
    service = {
        "service_id": barrier["service"]["service_id"],
        "position": copy.deepcopy(barrier["service"]["position"]),
        "capabilities": [
            {"capability_id": barrier["service"]["capability_id"], "kind": "restoration"}
        ],
    }
    return {
        "schema_version": 1,
        "kind": recording.FRAME_KIND,
        "provenance": {"server_sequence": "run-local", "world_revision": "run-local"},
        "frame_generation": 9,
        "frame": {
            "logical_time": str(barrier["minimum_logical_time"]),
            "observer_actor_id": barrier["observer_actor_id"],
            "observation_center": copy.deepcopy(actors[0]["position"]),
            "observation_radius": 7,
            "actors": actors,
            "action_options": [action],
            "action_options_truncated": False,
            "npcs_here": [
                {
                    "actor_id": barrier["npc_interaction"]["actor_id"],
                    "interactions": [interaction],
                }
            ],
            "services_here": [service],
            "static_scene_context": {
                "tiles": [{"terrain_ids": copy.deepcopy(barrier["required_terrain_ids"])}]
            },
            "tiles": [],
        },
    }


class TheTrackedArrangement(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = recording.load_fixture()
        source_path = REPOSITORY_ROOT / self.fixture["source"]["simulation_seed"]
        self.source = json.loads(source_path.read_text(encoding="utf-8"))

    def test_it_is_bound_to_the_seed_declared_by_the_world(self) -> None:
        declared = World.declared(self.fixture["source"]["world_document"])
        self.assertEqual(declared.simulation_seed, self.fixture["source"]["simulation_seed"])
        self.assertEqual(
            recording.sha256(REPOSITORY_ROOT / declared.simulation_seed),
            self.fixture["source"]["simulation_seed_sha256"],
        )

    def test_it_relocates_exactly_three_existing_facts(self) -> None:
        effective, deltas = recording.arrange_seed(self.source, self.fixture)
        self.assertEqual(
            [row["path"] for row in deltas],
            [
                "actors/threshold_keeper/location",
                "ecology_sites/ruin_mouth_lair/member_locations/great_bear",
                "service_instances/keeper_rite/location",
            ],
        )
        restored = copy.deepcopy(effective)
        next(row for row in restored["actors"] if row["id"] == "threshold_keeper")[
            "location"
        ] = deltas[0]["before"]
        next(row for row in restored["ecology_sites"] if row["id"] == "ruin_mouth_lair")[
            "member_locations"
        ]["great_bear"] = deltas[1]["before"]
        next(row for row in restored["service_instances"] if row["id"] == "keeper_rite")[
            "location"
        ] = deltas[2]["before"]
        self.assertEqual(restored, self.source)
        self.assertEqual(self.source["actors"][1]["location"], deltas[0]["before"])

    def test_realm_or_level_changes_are_refused(self) -> None:
        changed = copy.deepcopy(self.fixture)
        changed["arrangements"]["actors"][0]["location"]["realm"] = "somewhere_else"
        with self.assertRaises(ProofError) as refusal:
            recording.arrange_seed(self.source, changed)
        self.assertIn("inside its level and realm", str(refusal.exception))


class TheSemanticBarrier(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = recording.load_fixture()
        self.document = synthetic_frame(self.fixture)

    def test_the_complete_representative_frame_passes(self) -> None:
        projection = recording.validate_frame(self.document, self.fixture)
        self.assertEqual(3, len(projection["actors"]))
        self.assertEqual("player", projection["observer_actor_id"])

    def test_a_relabelled_keeper_cannot_substitute_for_the_hostile_actor(self) -> None:
        hostile = next(
            row for row in self.document["frame"]["actors"] if row["kind"] == "monster"
        )
        hostile["attack_safety"] = "protected"
        with self.assertRaises(ProofError) as refusal:
            recording.validate_frame(self.document, self.fixture)
        self.assertIn("attack_safety", str(refusal.exception))

    def test_the_service_must_remain_co_located_with_the_keeper(self) -> None:
        self.document["frame"]["services_here"][0]["position"]["position"]["x"] += 1
        with self.assertRaises(ProofError) as refusal:
            recording.validate_frame(self.document, self.fixture)
        self.assertIn("co-located", str(refusal.exception))

    def test_the_exact_enabled_interaction_must_be_server_supplied(self) -> None:
        self.document["frame"]["action_options"][0]["enabled"] = False
        with self.assertRaises(ProofError) as refusal:
            recording.validate_frame(self.document, self.fixture)
        self.assertIn("enabled", str(refusal.exception))

    def test_normalization_excludes_run_identity_but_keeps_gameplay_facts(self) -> None:
        first = recording.normalized_projection(self.document)
        rerun = copy.deepcopy(self.document)
        rerun["provenance"]["server_sequence"] = "another-run"
        rerun["frame_generation"] = 91
        rerun["frame"]["actors"][0]["character_id"] = "another-character"
        self.assertEqual(first, recording.normalized_projection(rerun))
        rerun["frame"]["actors"][0]["life_state"] = "dead"
        self.assertNotEqual(first, recording.normalized_projection(rerun))


class TheDriverAndClientContract(unittest.TestCase):
    def test_both_sides_name_the_same_script_and_sentinels(self) -> None:
        client_path = REPOSITORY_ROOT / "client/tests/record_presentation_adoption_frame.gd"
        source = client_path.read_text(encoding="utf-8")
        self.assertEqual("res://tests/record_presentation_adoption_frame.gd", recording.CLIENT_SCRIPT)
        self.assertIn(f'SUCCESS_SENTINEL: String = "{recording.CLIENT_SENTINEL}"', source)
        self.assertTrue(client_path.is_file())


class TheTrackedRecording(unittest.TestCase):
    def setUp(self) -> None:
        self.fixture = recording.load_fixture()
        self.frame_path = REPOSITORY_ROOT / recording.TRACKED_FRAME_PATH
        self.receipt_path = self.frame_path.with_suffix(".receipt.json")
        self.frame = json.loads(self.frame_path.read_text(encoding="utf-8"))
        self.receipt = json.loads(self.receipt_path.read_text(encoding="utf-8"))

    def test_it_is_the_real_server_and_shipped_client_output(self) -> None:
        provenance = self.frame["provenance"]
        self.assertEqual("headless_live_server", provenance["route"])
        self.assertEqual(
            "client/tests/record_presentation_adoption_frame.gd", provenance["recorded_by"]
        )
        self.assertEqual("tools/run_presentation_adoption_recording.py", provenance["driver"])
        self.assertEqual(self.receipt["source"]["commit"], provenance["source_commit"])
        self.assertEqual(self.receipt["source"]["tree"], provenance["source_tree"])
        recording.validate_frame(self.frame, self.fixture)

    def test_receipt_binds_every_tracked_source_and_observed_projection(self) -> None:
        source = self.receipt["source"]
        self.assertEqual([], source["tracked_status_before"])
        self.assertRegex(source["commit"], re.compile(r"^[0-9a-f]{40}$"))
        self.assertRegex(source["tree"], re.compile(r"^[0-9a-f]{40}$"))
        self.assertEqual(recording.sha256(self.frame_path), self.receipt["observed"]["frame_sha256"])
        projection = recording.normalized_projection(self.frame)
        self.assertEqual(
            recording.sha256_bytes(recording.canonical_json(projection)),
            self.receipt["observed"]["normalized_projection_sha256"],
        )
        self.assertEqual(
            recording.sha256(REPOSITORY_ROOT / recording.FIXTURE_PATH),
            source["recording_fixture_sha256"],
        )
        self.assertEqual(
            recording.sha256(REPOSITORY_ROOT / "tools/run_presentation_adoption_recording.py"),
            self.receipt["proof_sources"]["driver_sha256"],
        )
        self.assertEqual(
            recording.sha256(REPOSITORY_ROOT / "client/tests/record_presentation_adoption_frame.gd"),
            self.receipt["proof_sources"]["client_recorder_sha256"],
        )

    def test_the_receipt_cannot_be_misread_as_dead_layer_evidence(self) -> None:
        self.assertEqual(
            {
                "dead_layer": False,
                "static_prop_required": False,
                "transition_aperture_required": False,
            },
            self.receipt["evidence_limits"],
        )
        self.assertEqual(
            "prerequisite_candidate_pending_non_author_rerun", self.receipt["disposition"]
        )


if __name__ == "__main__":
    unittest.main()
