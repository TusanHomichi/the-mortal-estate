"""The restore-drill proof's own ownership, ordering and cleanup rules.

`tools/run_restore_drill_proof.py` is driven end to end by the gated PostgreSQL step.
These cases pin what a real run cannot show cheaply: that the cluster it touches must
be the one it created, that a cluster which will not stop is reported rather than
raised past a failure, that the coordinated writer runs inside the exported snapshot's
window and only there, and that a restored copy's credentials are re-pointed without
losing the role.
"""

import subprocess
import sys
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tools"))

import run_restore_drill_proof as proof  # noqa: E402


class Ownership(unittest.TestCase):
    """Only the cluster this proof created is ever touched."""

    class Installation(proof.ScratchInstallation):
        """The ownership check alone, without a cluster or a provisioning run."""

        def __init__(self, root, reported, data=None):
            self.root = Path(root).resolve()
            self.site = SimpleNamespace(
                data=Path(data) if data is not None else Path(root) / "postgres",
                socket=Path("/tmp/socket"),
                sql=lambda text, database="postgres": reported)

    def test_the_cluster_that_answers_must_keep_its_data_under_the_root(self):
        installation = self.Installation("/tmp/scratch", "/tmp/scratch/postgres")
        installation.assert_ownership()

    def test_a_cluster_answering_with_another_data_directory_is_refused(self):
        installation = self.Installation("/tmp/scratch", "/var/lib/postgresql/18/main")
        with self.assertRaisesRegex(proof.ProofError, "only touches the cluster it created"):
            installation.assert_ownership()

    def test_a_data_directory_outside_the_root_is_refused_before_anything_asks(self):
        installation = self.Installation("/tmp/scratch", "/tmp/scratch/postgres")
        installation.site.data = Path("/tmp/somewhere-else/postgres")
        with self.assertRaisesRegex(proof.ProofError, "outside"):
            installation.assert_ownership()


class StopReporting(unittest.TestCase):
    """Cleanup returns its problems, so a failed proof keeps its own failure."""

    class Installation(proof.ScratchInstallation):
        def __init__(self, started=True):
            self.started = started
            self.site = SimpleNamespace(data=Path("/tmp/scratch/postgres"))
            self.pg_bin = Path("/nonexistent")

    def test_a_cluster_that_never_started_is_not_stopped(self):
        with patch.object(proof, "run") as runner:
            self.assertEqual(self.Installation(started=False).close(), [])
        runner.assert_not_called()

    def test_a_cluster_that_will_not_stop_is_reported_not_raised(self):
        """A timeout is a cleanup problem too, not an exception past the failure."""
        with patch.object(proof, "run", side_effect=subprocess.TimeoutExpired("pg_ctl", 120)):
            problems = self.Installation().close()
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("was not stopped", problems[0])
        self.assertIn("/tmp/scratch/postgres", problems[0])
        self.assertIn("timed out", problems[0])


class PinnedReadWindow(unittest.TestCase):
    """The coordinated commit is placed, not raced."""

    def site(self, exporter_counts):
        counts = list(exporter_counts)
        return SimpleNamespace(sql=lambda text, database=None: str(counts.pop(0)))

    def test_the_writer_runs_once_when_the_first_pinned_read_starts(self):
        created = {"character_id": "c-committed"}
        calls = []

        def writer():
            calls.append("write")
            return created

        barrier = proof.PinnedReadBarrier(self.site([1, 1]), writer)
        with patch.object(proof, "exporting_sessions", side_effect=[1, 1]):
            barrier("BEGIN; SET TRANSACTION SNAPSHOT 'x'; SELECT 1;")
            barrier("BEGIN; SET TRANSACTION SNAPSHOT 'x'; SELECT 2;")
        barrier.check()
        self.assertEqual(calls, ["write"])
        self.assertEqual(barrier.created, created)

    def test_an_unpinned_read_is_not_a_window(self):
        barrier = proof.PinnedReadBarrier(self.site([]), lambda: {"character_id": "c"})
        barrier("SELECT count(*) FROM tme.characters")
        self.assertIsNone(barrier.created)
        with self.assertRaisesRegex(proof.ProofError, "without a snapshot to wait for"):
            barrier.check()

    def test_a_commit_outside_an_open_export_fails_the_check(self):
        """The claim is that the commit landed inside the exported snapshot."""
        barrier = proof.PinnedReadBarrier(self.site([]), lambda: {"character_id": "c"})
        with patch.object(proof, "exporting_sessions", side_effect=[0, 0]):
            barrier("BEGIN; SET TRANSACTION SNAPSHOT 'x'; SELECT 1;")
        with self.assertRaisesRegex(proof.ProofError, "inside an exported snapshot"):
            barrier.check()


class BarrierProxy(unittest.TestCase):
    """The proxy adds a hook; every other fact is the installation's own."""

    def test_sql_is_passed_through_with_its_arguments(self):
        seen = []

        def sql(text, database="tme", quiet=False):
            seen.append((text, database, quiet))
            return "rows"

        site = SimpleNamespace(root="the-root", pg_bin="the-bin", sql=sql)
        barrier = proof.PinnedReadBarrier(site, lambda: None)
        proxy = proof.SiteWithBarrier(site, barrier)
        self.assertEqual(proxy.sql("SELECT 1", database="other", quiet=True), "rows")
        self.assertEqual(seen, [("SELECT 1", "other", True)])
        self.assertEqual(proxy.root, "the-root")
        self.assertEqual(proxy.pg_bin, "the-bin")


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
        with self.assertRaises(proof.ProofError):
            proof.ScratchInstallation.url_for(installation, "tme_oracle_1", "database")


class FailureReporting(unittest.TestCase):
    """A refusal is only evidence when it is a refusal."""

    def test_a_refusal_is_returned_as_its_own_message(self):
        def refuses(site, directory):
            raise RuntimeError("restored database did not retain its backup: characters lost {x}")
        self.assertIn("characters lost", proof.expect_failure(refuses, None, None))

    def test_an_accepted_action_is_a_proof_failure(self):
        def accepts(site, directory):
            return None
        with self.assertRaisesRegex(proof.ProofError, "accepted evidence it must refuse"):
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
        with self.assertRaisesRegex(proof.ProofError, "2 times"):
            proof.position_of(self.frame([position, position]), "c-1")

    def test_an_absent_character_is_refused(self):
        with self.assertRaisesRegex(proof.ProofError, "0 times"):
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
        with self.assertRaisesRegex(proof.ProofError, "no passable square"):
            proof.take_one_step(gameplay, here, "c-1", timeout=5)
        self.assertEqual(gameplay.commands, [])


if __name__ == "__main__":
    unittest.main()
