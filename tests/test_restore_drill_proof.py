"""The restore-drill proof's own ownership, ordering and cleanup rules.

`tools/run_restore_drill_proof.py` is driven end to end by the gated PostgreSQL step.
These cases pin what a real run cannot show cheaply: that the cluster it touches must
be the one it created, that a cluster which will not stop is reported rather than
raised past a failure, that the coordinated writer runs inside the exported snapshot's
window and only there, and that a restored copy's credentials are re-pointed without
losing the role.
"""

import shutil
import subprocess
import sys
import tempfile
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
    """Cleanup returns its problems, and only a confirmed stop clears the obligation."""

    class Installation(proof.ScratchInstallation):
        def __init__(self, state):
            self.cluster_state = state
            self.site = SimpleNamespace(data=Path("/tmp/scratch/postgres"))
            self.pg_bin = Path("/nonexistent")

    def test_a_cluster_that_was_never_launched_is_not_stopped(self):
        with patch.object(proof, "run") as runner:
            self.assertEqual(self.Installation(proof.UNLAUNCHED).close(), [])
        runner.assert_not_called()

    def test_a_launch_that_failed_still_owes_a_stop(self):
        """`pg_ctl` can time out and come up anyway, so the obligation is not a flag."""
        with patch.object(proof, "run") as runner, \
                patch.object(proof.ScratchInstallation, "server_running", return_value=False):
            problems = self.Installation(proof.LAUNCH_ATTEMPTED).close()
        self.assertEqual(problems, [])
        self.assertEqual(runner.call_count, 1)
        self.assertIn("stop", runner.call_args.args[0])

    def test_a_server_that_survives_the_stop_is_reported_and_still_owed(self):
        """A timeout is a cleanup problem, and the next attempt must try again."""
        installation = self.Installation(proof.LAUNCH_ATTEMPTED)
        with patch.object(proof, "run", side_effect=subprocess.TimeoutExpired("pg_ctl", 120)), \
                patch.object(proof.ScratchInstallation, "server_running", return_value=True):
            problems = installation.close()
            self.assertEqual(len(problems), 1, problems)
            self.assertIn("is still running", problems[0])
            self.assertIn("/tmp/scratch/postgres", problems[0])
            self.assertIn("timed out", problems[0])
            self.assertEqual(installation.close(), problems)

    def test_a_confirmed_stop_clears_the_obligation(self):
        installation = self.Installation(proof.LAUNCH_ATTEMPTED)
        with patch.object(proof, "run") as runner, \
                patch.object(proof.ScratchInstallation, "server_running", return_value=False):
            self.assertEqual(installation.close(), [])
            self.assertEqual(installation.close(), [])
        self.assertEqual(runner.call_count, 1)

    def test_a_cluster_that_cannot_be_asked_whether_it_stopped_is_reported(self):
        with patch.object(proof, "run"), \
                patch.object(proof.ScratchInstallation, "server_running",
                             side_effect=OSError("pg_ctl is gone")):
            problems = self.Installation(proof.LAUNCH_ATTEMPTED).close()
        self.assertEqual(len(problems), 1, problems)
        self.assertIn("could not be asked whether it stopped", problems[0])


class ServerContext:
    """The live-server harness, as a context manager that starts nothing."""

    def __init__(self, *arguments, **keywords):
        self.arguments = arguments

    def __enter__(self):
        return SimpleNamespace()

    def __exit__(self, *_exception):
        return False


class ProofLifecycle(unittest.TestCase):
    """The outer proof removes its root only once no server holds it.

    These cases drive the real `proof()` with every external collaborator stubbed, so
    what they pin is this tool's own control flow: what it does with the root after a
    shutdown it could not confirm, and what it reports when the proof failed first.
    """

    class Installation:
        """A scratch installation whose lifecycle the outer proof controls."""

        def __init__(self, close_problems=(), provision_error=None):
            self.close_problems = list(close_problems)
            self.provision_error = provision_error
            self.closes = 0
            self.root = Path("/not-adopted")
            self.administrator = "proof-admin"
            self.site = SimpleNamespace(ports={"postgres": 1}, socket=Path("/tmp/socket"),
                                        config=Path("/tmp/config"))

        def adopt(self, root):
            """Take the temporary root the real proof just created."""
            self.root = Path(root)
            self.site = SimpleNamespace(ports={"postgres": 1}, socket=Path("/tmp/socket"),
                                        config=self.root / "config",
                                        sql=lambda text, database="postgres": "")
            return self

        def provision(self):
            if self.provision_error is not None:
                raise self.provision_error
            self.site.config.mkdir(parents=True, exist_ok=True)
            (self.site.config / "bootstrap.json").write_text(
                '{"characters": [{"character_id": "seeded-character"}]}', encoding="utf-8")

        def close(self):
            self.closes += 1
            return list(self.close_problems)

        def served(self, character_id, database=proof.SCRATCH_DATABASE, account=0):
            return SimpleNamespace(database_name=database, character_id=character_id)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="tme-proof-lifecycle-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def run_proof(self, installation, *, keep=False, stages=None):
        """The root removals the proof requested, and the failure it surfaced."""
        stages = stages or {}
        removed = []
        world = SimpleNamespace(world_template="world.json")
        preserved = {"saved": Path("/backups/one"), "created": {"character_id": "c-1"},
                     "position": {}}

        def stage_of(name, default):
            value = stages.get(name, default)
            if isinstance(value, BaseException):
                def raises(*arguments, **keywords):
                    raise value
                return raises
            return lambda *arguments, **keywords: value

        arguments = SimpleNamespace(postgres_bin="/fake/bin", world_document="world.json",
                                    keep=keep)
        with patch.object(proof, "postgres_binaries", return_value=Path("/fake/bin")), \
                patch.object(proof, "build_server", return_value=Path("/fake/bin/tme-server")), \
                patch.object(proof, "World", SimpleNamespace(declared=lambda *a, **k: world)), \
                patch.object(proof, "ScratchInstallation",
                             side_effect=lambda root, binary, document, pg_bin: installation.adopt(root)), \
                patch.object(proof, "LiveServer", ServerContext), \
                patch.object(proof.shutil, "rmtree",
                             side_effect=lambda path, **keywords: removed.append(Path(path))), \
                patch.object(proof, "prove_preservation",
                             side_effect=stage_of("preserve", preserved)), \
                patch.object(proof, "prove_concurrent_commit",
                             side_effect=stage_of("concurrent", None)), \
                patch.object(proof, "prove_restored_position",
                             side_effect=stage_of("oracle", None)), \
                patch.object(proof, "prove_rejection",
                             side_effect=stage_of("rejection", Path("/backups/altered"))), \
                patch.object(proof, "prove_cleanup_failure",
                             side_effect=stage_of("cleanup", None)):
            try:
                proof.proof(arguments)
                failure = None
            except BaseException as error:
                failure = error
        # The real root was created by the real `tempfile`; only its removal was
        # intercepted, so the test owns deleting it.
        self.addCleanup(shutil.rmtree, installation.root, ignore_errors=True)
        return removed, failure

    def test_a_confirmed_shutdown_removes_the_root(self):
        installation = self.Installation()
        removed, failure = self.run_proof(installation)
        self.assertIsNone(failure)
        self.assertEqual(removed, [installation.root])
        self.assertEqual(installation.closes, 1)

    def test_an_unresolved_shutdown_retains_the_root(self):
        """The files are what somebody needs to finish or diagnose the job."""
        problem = "the scratch cluster in /tmp/x/postgres is still running"
        installation = self.Installation(close_problems=[problem])
        removed, failure = self.run_proof(installation)
        self.assertEqual(removed, [], "a root whose server survived must not be deleted")
        self.assertIsInstance(failure, proof.ProofError)
        self.assertIn("was retained", str(failure))
        self.assertIn(problem, str(failure))
        self.assertIn(str(installation.root), str(failure))

    def test_a_primary_failure_with_an_unresolved_shutdown_retains_the_root(self):
        """Both facts survive: what went wrong, and where the cluster still is."""
        primary = proof.ProofError("the created character was not preserved")
        problem = "the scratch cluster in /tmp/x/postgres is still running"
        installation = self.Installation(close_problems=[problem])
        removed, failure = self.run_proof(installation, stages={"preserve": primary})
        self.assertEqual(removed, [])
        self.assertIs(failure.__cause__, primary)
        self.assertIn("the created character was not preserved", str(failure))
        self.assertIn("was retained", str(failure))
        self.assertIn(problem, str(failure))

    def test_a_primary_failure_with_a_confirmed_shutdown_removes_the_root(self):
        primary = proof.ProofError("the created character was not preserved")
        installation = self.Installation()
        removed, failure = self.run_proof(installation, stages={"preserve": primary})
        self.assertIs(failure, primary)
        self.assertEqual(removed, [installation.root])

    def test_a_provision_failure_before_any_launch_removes_the_root(self):
        """Nothing was launched, so there is nothing to keep the files for."""
        installation = self.Installation(provision_error=RuntimeError("injected initdb failure"))
        removed, failure = self.run_proof(installation)
        self.assertIsInstance(failure, RuntimeError)
        self.assertIn("injected initdb failure", str(failure))
        self.assertEqual(removed, [installation.root])

    def test_keep_retains_the_root_without_a_failure(self):
        installation = self.Installation()
        removed, failure = self.run_proof(installation, keep=True)
        self.assertIsNone(failure)
        self.assertEqual(removed, [])


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
