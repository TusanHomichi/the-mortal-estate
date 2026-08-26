"""Tests for the live proof's judgement of WHICH land it signed in to.

The live proof needs PostgreSQL, TLS, credentials, and the pinned Godot binary,
so it runs on demand. Two things it decides need none of that and must not go
soft: which world it serves, and whether the client ended up standing in it.

A proof that reported success while the runtime served some other land — a
fixture, a corpus scenario, whatever the tree happens to carry — would be a
green light for the one claim slice S1 exists to make.
"""

from __future__ import annotations

import contextlib
import io
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

import boundary_test_support  # noqa: F401  (puts tools/ on sys.path)
import run_client_live_proof as live_proof
from live_server_harness import REPOSITORY_ROOT, ProofError, World

PROOF_DOCUMENT = "content/lands/identity-proof/world.json"


def centre(x: int, y: int) -> str:
    return f"shell = Observation centre {x},{y} · frame generation 2 · logical time 4"


def judge(stdout: str, world: World) -> str:
    captured = io.StringIO()
    with contextlib.redirect_stdout(captured):
        with TemporaryDirectory() as run_directory:
            live_proof.check_land(stdout, world, Path(run_directory))
    return captured.getvalue()


class TheServedWorldIsReadFromTheLand(unittest.TestCase):
    def test_the_tracked_document_resolves_to_files_that_exist(self) -> None:
        world = World.declared(PROOF_DOCUMENT)
        for named in (world.world_template, world.simulation_seed, world.catalog):
            self.assertTrue((REPOSITORY_ROOT / named).is_file(), named)
        self.assertEqual(
            world.world_template, "content/lands/identity-proof/generated/world_template.json"
        )
        self.assertEqual(world.controlled_actor, "player")

    def test_the_land_the_proof_serves_is_the_compilers_own_output(self) -> None:
        """The whole point of S1: the served template is compiled, not hand-authored.

        A template that stopped being the compiler's emission would mean a
        Workbench edit could no longer reach play, which is the failure this
        binding exists to make loud.
        """
        world = World.declared(PROOF_DOCUMENT)
        template = json.loads(
            (REPOSITORY_ROOT / world.world_template).read_text(encoding="utf-8")
        )
        self.assertEqual(template["id"], "identity_proof")
        self.assertIn("generated/", world.world_template)

    def test_a_document_of_another_kind_is_refused(self) -> None:
        with TemporaryDirectory() as directory:
            path = Path(directory) / "world.json"
            path.write_text(json.dumps({"schema_version": 1, "kind": "something_else"}))
            with self.assertRaises(ProofError):
                World.declared(str(path.relative_to(REPOSITORY_ROOT))
                               if path.is_relative_to(REPOSITORY_ROOT) else str(path))

    def test_a_document_naming_content_that_is_not_there_is_refused(self) -> None:
        source = json.loads((REPOSITORY_ROOT / PROOF_DOCUMENT).read_text(encoding="utf-8"))
        source["simulation_seed"] = "no-such-seed.json"
        directory = REPOSITORY_ROOT / "content/lands/identity-proof"
        path = directory / "world-under-test.json"
        path.write_text(json.dumps(source), encoding="utf-8")
        self.addCleanup(path.unlink)
        with self.assertRaises(ProofError) as caught:
            World.declared(str(path.relative_to(REPOSITORY_ROOT)))
        self.assertIn("simulation_seed", str(caught.exception))


class TheClientMustBeStandingInTheServedLand(unittest.TestCase):
    def setUp(self) -> None:
        self.world = World.declared(PROOF_DOCUMENT)
        seed = json.loads(
            (REPOSITORY_ROOT / self.world.simulation_seed).read_text(encoding="utf-8")
        )
        controlled = next(
            actor for actor in seed["actors"] if actor["id"] == self.world.controlled_actor
        )
        self.seated = (
            controlled["location"]["position"]["x"],
            controlled["location"]["position"]["y"],
        )

    def test_the_seeded_square_is_accepted_and_named(self) -> None:
        report = judge(centre(*self.seated), self.world)
        self.assertIn(f"observation centre {self.seated[0]},{self.seated[1]}", report)
        self.assertIn("identity_proof/settlement", report)
        self.assertIn("threshold_keeper", report)

    def test_another_lands_square_is_refused(self) -> None:
        """The corpus land seats its player at 25,62. That must not pass here."""
        with self.assertRaises(ProofError) as caught:
            judge(centre(25, 62), self.world)
        self.assertIn("25,62", str(caught.exception))
        self.assertIn(str(self.seated[0]), str(caught.exception))

    def test_a_client_that_reported_no_centre_is_refused(self) -> None:
        with self.assertRaises(ProofError) as caught:
            judge("ok: signed in\nlifecycle = online", self.world)
        self.assertIn("no observation centre", str(caught.exception))


if __name__ == "__main__":
    unittest.main()
