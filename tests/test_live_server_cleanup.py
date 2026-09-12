"""Failed live-proof cleanup remains observable and owns only its own resources."""

import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, call, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

from live_server_harness import (  # noqa: E402
    CLEANUP_TIMEOUT_SECONDS, SERVER_EXIT_TIMEOUT_SECONDS, LiveServer, ProofError, World,
)


class LiveServerCleanup(unittest.TestCase):
    def server(self, *, keep=False, owns_database=True):
        server = LiveServer("postgresql://operator:p%40ssword@localhost/postgres",
                            World("unused"), keep=keep)
        server.run_directory = Path(tempfile.mkdtemp(prefix="tme-cleanup-test-"))
        self.addCleanup(shutil.rmtree, server.run_directory, ignore_errors=True)
        server.server_log = server.run_directory / "server.log"
        server.server_log.write_text("diagnostic evidence\n")
        server.database_name = "tme_live_owned_test"
        server.database_url = "postgresql://runtime:runtime-secret@localhost/tme_live_owned_test"
        server.password = "private-test-password"
        server._database_cleanup_due = owns_database
        return server

    def completed(self, code=0, stderr=""):
        return subprocess.CompletedProcess([], code, "", stderr)

    def test_success_drops_only_the_owned_database_and_is_idempotent(self):
        server = self.server()
        with patch("live_server_harness.subprocess.run", return_value=self.completed()) as run:
            server.close()
            server.close()
        run.assert_called_once()
        self.assertEqual(run.call_args.args[0][-1],
                         'DROP DATABASE IF EXISTS "tme_live_owned_test" WITH (FORCE)')
        self.assertEqual(run.call_args.kwargs["timeout"], CLEANUP_TIMEOUT_SECONDS)
        self.assertFalse(server._database_cleanup_due)
        self.assertFalse(server.run_directory.exists())

    def test_no_creation_attempt_means_no_database_cleanup(self):
        server = self.server(owns_database=False)
        with patch("live_server_harness.subprocess.run") as run:
            server.close()
        run.assert_not_called()
        self.assertFalse(server.run_directory.exists())

    def test_failed_drop_retains_evidence_and_retries_without_leaking_credentials(self):
        server = self.server()
        diagnostic = f"connection {server.admin_url}: p@ssword; {server.database_url}"
        with patch("live_server_harness.subprocess.run",
                   side_effect=[self.completed(1, diagnostic), self.completed()]) as run:
            with self.assertRaises(ProofError) as caught:
                server.close()
            message = str(caught.exception)
            self.assertIn(server.database_name, message)
            self.assertIn(str(server.run_directory), message)
            self.assertIn("exited 1", message)
            for secret in (server.admin_url, "p%40ssword", "p@ssword", "runtime-secret"):
                self.assertNotIn(secret, message)
            self.assertTrue(server._database_cleanup_due)
            self.assertEqual(server.server_log.read_text(), "diagnostic evidence\n")
            server.close()
        self.assertEqual(run.call_count, 2)
        self.assertFalse(server.run_directory.exists())

    def test_timed_out_drop_names_the_owned_resource_and_remains_owed(self):
        server = self.server()
        error = subprocess.TimeoutExpired(["psql", server.admin_url], CLEANUP_TIMEOUT_SECONDS,
                                          stderr=f"waiting on {server.admin_url}")
        with patch("live_server_harness.subprocess.run", side_effect=error):
            with self.assertRaises(ProofError) as caught:
                server.close()
        self.assertIn("exceeded", str(caught.exception))
        self.assertIn(server.database_name, str(caught.exception))
        self.assertNotIn("p%40ssword", str(caught.exception))
        self.assertTrue(server._database_cleanup_due)
        self.assertTrue(server.server_log.is_file())

    def test_forced_termination_is_reaped_and_reported(self):
        server = self.server()
        process = Mock()
        process.poll.return_value = None
        process.wait.side_effect = [subprocess.TimeoutExpired("server", 20), -9]
        server._server = process
        with patch("live_server_harness.subprocess.run", return_value=self.completed()):
            with self.assertRaisesRegex(ProofError, "forced termination and was reaped"):
                server.close()
        process.terminate.assert_called_once()
        process.kill.assert_called_once()
        self.assertEqual(process.wait.call_args_list,
                         [call(timeout=SERVER_EXIT_TIMEOUT_SECONDS)] * 2)
        self.assertIsNone(server._server)
        self.assertTrue(server.server_log.is_file())

    def test_failed_kill_or_reaping_keeps_the_process_and_database_for_retry(self):
        for kill_error in (OSError("kill denied"), None):
            with self.subTest(kill_error=kill_error):
                server = self.server()
                process = Mock()
                process.poll.return_value = None
                process.wait.side_effect = subprocess.TimeoutExpired("server", 20)
                process.kill.side_effect = kill_error
                server._server = process
                with patch("live_server_harness.subprocess.run") as run:
                    with self.assertRaisesRegex(ProofError, "could not be reaped"):
                        server.close()
                run.assert_not_called()
                self.assertIs(server._server, process)
                self.assertTrue(server._database_cleanup_due)
                self.assertTrue(server.server_log.is_file())
                process.poll.return_value = -9
                process.wait.side_effect = None
                process.wait.return_value = -9
                with patch("live_server_harness.subprocess.run", return_value=self.completed()):
                    server.close()
                self.assertFalse(server.run_directory.exists())

    def test_keep_stops_the_server_but_keeps_the_database_and_directory(self):
        server = self.server(keep=True)
        process = Mock()
        process.poll.return_value = None
        server._server = process
        with patch("live_server_harness.subprocess.run") as run:
            server.close()
        run.assert_not_called()
        process.wait.assert_called_once_with(timeout=SERVER_EXIT_TIMEOUT_SECONDS)
        self.assertIsNone(server._server)
        self.assertTrue(server._database_cleanup_due)
        self.assertTrue(server.server_log.exists())

    def test_keep_does_not_hide_a_server_that_cannot_be_reaped(self):
        server = self.server(keep=True)
        server._server = Mock()
        server._server.poll.return_value = None
        server._server.terminate.side_effect = OSError("termination denied")
        with patch("live_server_harness.subprocess.run") as run:
            with self.assertRaisesRegex(ProofError, "could not be reaped"):
                server.close()
        run.assert_not_called()
        self.assertTrue(server.server_log.is_file())

    def test_proxy_failure_keeps_diagnostics_and_does_not_drop_the_database(self):
        server = self.server()
        proxy = Mock()
        proxy.close.side_effect = OSError("proxy remains")
        server._proxy = proxy
        with patch("live_server_harness.subprocess.run") as run:
            with self.assertRaisesRegex(ProofError, "proxy could not stop"):
                server.close()
        run.assert_not_called()
        self.assertIs(server._proxy, proxy)
        self.assertTrue(server.server_log.is_file())

    def test_root_deletion_failure_is_reported_after_database_cleanup(self):
        server = self.server()
        with patch("live_server_harness.subprocess.run", return_value=self.completed()), \
                patch("live_server_harness.shutil.rmtree", side_effect=PermissionError("denied")):
            with self.assertRaisesRegex(ProofError, "run directory could not be removed"):
                server.close()
        self.assertFalse(server._database_cleanup_due)
        self.assertTrue(server.server_log.is_file())

    def test_proof_and_provisioning_failures_survive_failed_cleanup(self):
        for fail_during_provisioning in (False, True):
            with self.subTest(provisioning=fail_during_provisioning):
                server = LiveServer("postgresql://operator@localhost/postgres", World("unused"))
                primary = RuntimeError("the original proof failed")

                def provision():
                    server._database_cleanup_due = True
                    if fail_during_provisioning:
                        raise primary

                with patch.object(server, "_provision", side_effect=provision), \
                        patch("live_server_harness.subprocess.run",
                              return_value=self.completed(1, "database still in use")):
                    with self.assertRaises(ProofError) as caught:
                        with server:
                            raise primary
                self.addCleanup(shutil.rmtree, server.run_directory, ignore_errors=True)
                self.assertIs(caught.exception.__cause__, primary)
                self.assertIn(str(primary), str(caught.exception))
                self.assertIn("database still in use", str(caught.exception))
                self.assertIn(server.database_name, str(caught.exception))
                self.assertTrue(server.run_directory.exists())


if __name__ == "__main__":
    unittest.main()
