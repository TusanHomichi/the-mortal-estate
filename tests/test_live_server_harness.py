"""Provisioning hands one resolved private denylist to every child it starts.

`tools/live_server_harness.py` is the only place that decides the
`TME_BANNED_TERMS_FILE` its child processes read. It had hardcoded a
checkout-local path, so a linked worktree — which must never carry the ignored
private list — failed the whole live proof there while `--allow-unavailable`
could still report a green job. It had also supplied the value only to the
served process: the offline `migrate`, `account create` and `bootstrap verify`
invocations received none at all, which is the failure actually observed.

These cases pin the contract rather than the implementation: provisioning is
exercised with every external boundary stubbed, and the environment each child
is actually launched with is captured and asserted. A refactor that stops
resolving the shared path, drops it from one call, or lets an inherited value win
fails here. The resolver itself is not reimplemented or re-tested;
`tests/test_check_boundary_terms.py` owns its behavior, including checkout-local
precedence, the linked-worktree fallback, and the fail-closed path when neither
list exists.
"""

import os
import shutil
import tempfile
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

from boundary_common import PRIVATE_TERMS_RELATIVE
from live_server_harness import LiveServer, World

ADMIN_URL = "postgresql://operator@127.0.0.1:5432/postgres"


class ProvisioningEnvironment(unittest.TestCase):
    """One synthetic linked worktree whose main checkout holds the only list."""

    def setUp(self) -> None:
        super().setUp()
        self.main = Path(tempfile.mkdtemp(prefix="tme-main-")).resolve()
        (self.main / ".git" / "worktrees" / "bridge").mkdir(parents=True)
        self.addCleanup(shutil.rmtree, self.main, ignore_errors=True)
        self.worktree = Path(tempfile.mkdtemp(prefix="tme-wt-")).resolve()
        self.addCleanup(shutil.rmtree, self.worktree, ignore_errors=True)
        (self.worktree / ".git").write_text(
            f"gitdir: {self.main / '.git' / 'worktrees' / 'bridge'}\n", encoding="utf-8"
        )
        self.shared_terms = self.main / PRIVATE_TERMS_RELATIVE
        self.shared_terms.parent.mkdir(parents=True)
        self.shared_terms.write_text("zorbelquux\n", encoding="utf-8")
        self.offline_runs: list[tuple[list[str], dict[str, str]]] = []
        self.served_runs: list[tuple[list[str], dict[str, str]]] = []

    def recorded_run(self, command, *, env=None, stdin=None):
        self.offline_runs.append((list(command), dict(env or {})))
        if command[0] == "psql":
            return ""
        # `account create` prints the created identifier as its last line.
        return "created-account-id"

    def recorded_popen(self, command, **kwargs):
        self.served_runs.append((list(command), dict(kwargs.get("env") or {})))
        handle = Mock()
        handle.poll.return_value = None
        return handle

    def provision(self) -> LiveServer:
        """Provision a server with every external boundary stubbed."""
        # The world is irrelevant here: the manifest is written, never served.
        server = LiveServer(
            ADMIN_URL, World(world_template="world.json", generated_seed={"actors": []})
        )
        binary = self.worktree / "tme-server"
        binary.write_text("#!/bin/sh\n", encoding="utf-8")
        binary.chmod(0o755)
        with (
            patch("live_server_harness.REPOSITORY_ROOT", self.worktree),
            patch("live_server_harness.build_server", return_value=binary),
            patch("live_server_harness.run", side_effect=self.recorded_run),
            patch("live_server_harness.reserve_port", side_effect=[41001, 41002, 41003]),
            patch(
                "live_server_harness.create_certificates",
                return_value=(binary, binary, binary),
            ),
            patch("live_server_harness.TlsProxy", return_value=Mock()),
            patch("live_server_harness.subprocess.Popen", side_effect=self.recorded_popen),
            # Readiness is polled over HTTP against the real process. That wait is
            # not this contract's subject, and the served child is stubbed above.
            patch.object(
                LiveServer,
                "_wait_for_ready",
                return_value={"gameplay_ready": True, "protocol_major": 1, "protocol_minor": 10},
            ),
        ):
            server.__enter__()
        self.addCleanup(shutil.rmtree, server.run_directory, ignore_errors=True)
        return server

    def children(self) -> dict[str, dict[str, str]]:
        """Key every captured child by its subcommand, not by the order it ran in."""
        started: dict[str, dict[str, str]] = {}
        for command, environment in [*self.offline_runs, *self.served_runs]:
            if Path(command[0]).name == "psql":
                continue
            started[" ".join(command[1:3])] = environment
        return started

    def assert_denylist(self, expected: str, why: str) -> None:
        children = self.children()
        self.assertEqual(
            sorted(children),
            ["account create", "bootstrap verify", "migrate", "serve"],
            "provisioning must start exactly these children for this contract to be complete",
        )
        for name, environment in children.items():
            self.assertEqual(environment.get("TME_BANNED_TERMS_FILE"), expected, f"{name}: {why}")

    def test_every_child_receives_the_worktree_aware_denylist(self) -> None:
        self.provision()
        self.assert_denylist(
            str(self.shared_terms),
            "must read the same private denylist the boundary check resolves",
        )

    def test_an_inherited_denylist_cannot_override_the_resolved_path(self) -> None:
        """The explicit value must beat `**os.environ` however the merge is written."""
        with patch.dict(os.environ, {"TME_BANNED_TERMS_FILE": str(self.worktree / "stale.txt")}):
            self.provision()
        self.assert_denylist(
            str(self.shared_terms),
            "inherited a conflicting denylist instead of the resolved one",
        )

    def test_an_absent_denylist_fails_closed_on_the_local_path(self) -> None:
        """No list anywhere must not become an empty value or a synthetic fallback."""
        self.shared_terms.unlink()
        self.provision()
        self.assert_denylist(
            str(self.worktree / PRIVATE_TERMS_RELATIVE),
            "must fail closed on the missing local path, not stand in for it",
        )

    def test_provisioning_does_not_mutate_the_process_environment(self) -> None:
        before = dict(os.environ)
        self.provision()
        self.assertEqual(dict(os.environ), before)

    def test_provisioning_adds_the_scratch_url_only_to_the_offline_children(self) -> None:
        """The denylist fix must not disturb database or credential wiring.

        The served environment is built with `**os.environ`, so an inherited
        `DATABASE_URL` would survive into it and this case would otherwise be
        asserting a property of the ambient shell rather than of provisioning.
        Control the parent environment, and state the claim precisely: the
        harness *adds* the scratch URL to the offline commands and does not add it
        to the served process, which reads it from its credential directory.
        """
        with patch.dict(os.environ):
            os.environ.pop("DATABASE_URL", None)
            server = self.provision()
        children = self.children()
        for name in ("migrate", "account create", "bootstrap verify"):
            self.assertEqual(
                children[name].get("DATABASE_URL"),
                server.database_url,
                f"{name} must still receive the scratch database URL",
            )
        served = children["serve"]
        self.assertNotEqual(
            served.get("DATABASE_URL"),
            server.database_url,
            "the served process reads its database URL from the credential directory",
        )
        self.assertIsNotNone(served.get("CREDENTIALS_DIRECTORY"))
        self.assertEqual(served.get("TME_PUBLIC_ORIGIN"), server.origin)

    def test_an_inherited_database_url_is_not_overwritten(self) -> None:
        """Inheritance is passed through unchanged; only the scratch URL is new.

        Recorded as current behavior rather than endorsed: the harness does not
        strip an inherited value, and nothing here asks it to. If that becomes a
        desired behavior it is a separate decision with its own coverage.
        """
        inherited = "postgresql://inherited@127.0.0.1:5432/inherited"
        with patch.dict(os.environ, {"DATABASE_URL": inherited}):
            server = self.provision()
        children = self.children()
        self.assertEqual(children["serve"].get("DATABASE_URL"), inherited)
        for name in ("migrate", "account create", "bootstrap verify"):
            self.assertEqual(
                children[name].get("DATABASE_URL"),
                server.database_url,
                f"{name} must still prefer the scratch database URL",
            )


if __name__ == "__main__":
    unittest.main()
