"""The restore-drill proof's own refusal and cleanup rules, without a database.

`tools/run_restore_drill_proof.py` is driven end to end by the gated PostgreSQL step.
These cases pin the parts a real run cannot show cheaply: that an installed cluster is
refused before anything is created, that a restored copy's credentials are re-pointed
without losing the role, and that teardown reports what it could not remove instead of
raising past whatever the proof was already reporting.
"""

import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

import run_restore_drill_proof as proof  # noqa: E402
from run_gated_postgres import GatedError  # noqa: E402


class Refusal(unittest.TestCase):
    """A cluster holding the instrument database is never touched."""

    class Cluster:
        def __init__(self, held):
            self.held = held
            self.asked = []

        def psql(self, statement, database=None):
            self.asked.append(statement)
            return self.held

    def test_an_installed_cluster_is_refused_before_anything_is_created(self):
        cluster = self.Cluster("1\n")
        with self.assertRaisesRegex(GatedError, "not a scratch cluster"):
            proof.refuse_an_installed_cluster(cluster)
        self.assertEqual(len(cluster.asked), 1)
        self.assertIn(proof.SCRATCH_DATABASE, cluster.asked[0])

    def test_an_empty_cluster_is_accepted(self):
        cluster = self.Cluster("0\n")
        proof.refuse_an_installed_cluster(cluster)
        self.assertEqual(len(cluster.asked), 1)


class CredentialRepointing(unittest.TestCase):
    """A restored database is reached with the installation's own role."""

    def installation(self):
        return SimpleNamespace(
            credentials={
                "database": "postgresql://tme_runtime:secret@localhost:5432/tme?host=%2Ftmp%2Fsock",
                "auth": "postgresql://tme_auth:secret@localhost:5432/tme?host=%2Ftmp%2Fsock",
            })

    def test_a_restored_database_keeps_the_role_and_the_socket(self):
        installation = self.installation()
        repointed = proof.ScratchInstallation.url_for(installation, "tme_oracle_1", "database")
        self.assertEqual(
            repointed,
            "postgresql://tme_runtime:secret@localhost:5432/tme_oracle_1?host=%2Ftmp%2Fsock")

    def test_a_credential_that_names_no_installation_database_is_refused(self):
        installation = SimpleNamespace(credentials={"database": "postgresql://host/other"})
        with self.assertRaises(GatedError):
            proof.ScratchInstallation.url_for(installation, "tme_oracle_1", "database")


class TeardownReporting(unittest.TestCase):
    """Cleanup returns its problems, so a failed proof keeps its own failure."""

    def installation(self, site_error=None, role_error=None):
        installation = SimpleNamespace(
            created_roles={"tme_owner", "tme_runtime"},
            site=SimpleNamespace(sql=self.site_sql(site_error)),
            cluster=SimpleNamespace(psql=self.cluster_psql(role_error)))
        return installation

    def site_sql(self, error):
        def sql(statement, database="tme"):
            if error is not None:
                raise error
            return ""
        return sql

    def cluster_psql(self, error):
        def psql(statement, database=None):
            if error is not None:
                raise error
            return ""
        return psql

    def test_a_dropped_database_and_its_roles_report_no_problems(self):
        problems = proof.ScratchInstallation.close(self.installation())
        self.assertEqual(problems, [])

    def test_a_role_that_will_not_drop_is_reported_not_raised(self):
        problems = proof.ScratchInstallation.close(
            self.installation(role_error=RuntimeError("privileges for function pg_control_system()")))
        self.assertEqual(len(problems), 2, problems)
        self.assertTrue(all("dropping role" in problem for problem in problems), problems)
        self.assertTrue(any("pg_control_system" in problem for problem in problems), problems)

    def test_a_surviving_instrument_database_is_reported_not_raised(self):
        problems = proof.ScratchInstallation.close(
            self.installation(site_error=RuntimeError("database is being accessed by other users")))
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("dropping tme failed", problems[0])


class FailureReporting(unittest.TestCase):
    """A refusal is only evidence when it is a refusal."""

    def test_a_refusal_is_returned_as_its_own_message(self):
        def refuses(site, directory):
            raise RuntimeError("restored database did not retain its backup: characters lost {x}")
        self.assertIn("characters lost", proof.expect_failure(refuses, None, None))

    def test_an_accepted_action_is_a_proof_failure(self):
        def accepts(site, directory):
            return None
        with self.assertRaisesRegex(GatedError, "accepted evidence it must refuse"):
            proof.expect_failure(accepts, None, None)


class PositionReading(unittest.TestCase):
    """The oracle reads the actor's own row, and refuses an ambiguous frame."""

    def frame(self, positions):
        return {"contract_version": proof.OBSERVER_CONTRACT_VERSION,
                "actors": [{"character_id": "c-1", "position": position}
                           for position in positions]}

    def test_the_named_actor_position_is_returned(self):
        expected = {"realm": "first_expedition", "level": "arrival",
                    "position": {"x": 8, "y": 33}}
        self.assertEqual(proof.position_of(self.frame([expected]), "c-1"), expected)

    def test_a_frame_that_names_the_character_twice_is_refused(self):
        position = {"realm": "first_expedition", "level": "arrival",
                    "position": {"x": 8, "y": 33}}
        with self.assertRaisesRegex(GatedError, "2 times"):
            proof.position_of(self.frame([position, position]), "c-1")

    def test_an_absent_character_is_refused(self):
        with self.assertRaisesRegex(GatedError, "0 times"):
            proof.position_of(self.frame([]), "c-1")


class Stepping(unittest.TestCase):
    """A step is taken only onto a square the server itself calls passable."""

    class Gameplay:
        def __init__(self, frames):
            self.frames = list(frames)
            self.commands = []
            self.latest_state = {}
            self.socket = SimpleNamespace(settimeout=lambda _timeout: None)

        def command(self, intent):
            self.commands.append(intent)
            return {"disposition": {"kind": "accepted"}}, intent

        def receive_json(self):
            value = self.frames.pop(0) if self.frames else {"kind": "heartbeat"}
            if "frame" in value:
                self.latest_state = value
            return value

    def frame(self, x, y, passable):
        return {"contract_version": proof.OBSERVER_CONTRACT_VERSION,
                "actors": [{"character_id": "c-1",
                            "position": {"realm": "r", "level": "l", "position": {"x": x, "y": y}}}],
                "tiles": [{"position": {"x": square[0], "y": square[1]}, "passable": True}
                          for square in passable]}

    def latest(self, frame):
        return {"frame": frame}

    def test_a_step_goes_to_a_passable_neighbour_and_waits_for_the_new_position(self):
        here = self.frame(8, 34, [(8, 33)])
        there = self.frame(8, 33, [(8, 33)])
        gameplay = self.Gameplay([self.latest(there)])
        moved = proof.take_one_step(gameplay, here, "c-1", timeout=5)
        self.assertEqual(gameplay.commands, [{"kind": "move_path", "path": ["north"]}])
        self.assertEqual(proof.position_of(moved, "c-1")["position"], {"x": 8, "y": 33})

    def test_a_character_with_no_passable_neighbour_is_not_moved(self):
        here = self.frame(8, 34, [])
        gameplay = self.Gameplay([])
        with self.assertRaisesRegex(GatedError, "no passable square"):
            proof.take_one_step(gameplay, here, "c-1", timeout=5)
        self.assertEqual(gameplay.commands, [])


if __name__ == "__main__":
    unittest.main()
