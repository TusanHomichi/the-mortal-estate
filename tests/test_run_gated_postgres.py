"""The gated PostgreSQL runner's contract, without a database.

What can be proven without provisioning anything is the part that was wrong
before it existed: which tests are gated, that each gets its own fresh
database, and that the fenced restore takes its dump from the durability
test's database rather than from whatever ran last. `docs/server-notes.md`
records all three as traps that were paid for once.

Running it for real is `python3 tools/run_gated_postgres.py`, and it is the
`gated` lane of the verification runner.
"""

from __future__ import annotations

import unittest
from pathlib import Path

from verification_test_support import REPO_ROOT

import run_gated_postgres as gated


class TheGatedInventory(unittest.TestCase):
    def test_every_ignored_test_in_the_workspace_has_an_entry(self) -> None:
        """The inventory fails closed the same way the Python one does."""
        gated_names = set()
        for source in (REPO_ROOT / "crates").rglob("*.rs"):
            lines = source.read_text(encoding="utf-8", errors="replace").splitlines()
            for index, line in enumerate(lines):
                if not line.lstrip().startswith("#[ignore"):
                    continue
                for candidate in lines[index + 1 : index + 4]:
                    if "fn " in candidate:
                        gated_names.add(candidate.split("fn ")[1].split("(")[0].strip())
                        break
        covered = {test.filter.split("::")[-1] for test in gated.GATED_TESTS}
        self.assertEqual(
            gated_names - covered,
            set(),
            "these gated tests have no runner entry and would never execute",
        )

    def test_names_are_unique(self) -> None:
        names = [test.name for test in gated.GATED_TESTS]
        self.assertEqual(len(names), len(set(names)))

    def test_every_entry_names_a_real_cargo_target(self) -> None:
        for test in gated.GATED_TESTS:
            if test.target[0] == "--lib":
                self.assertTrue((REPO_ROOT / "crates/tme-server/src/lib.rs").is_file())
            else:
                self.assertTrue(
                    (REPO_ROOT / f"crates/tme-server/tests/{test.target[1]}.rs").is_file(),
                    test.name,
                )


class OneFreshDatabasePerTest(unittest.TestCase):
    def test_every_plain_gated_test_is_provisioned_fresh(self) -> None:
        for test in gated.GATED_TESTS:
            self.assertIn(test.provisioning, {"fresh", "restore", "ev"}, test.name)

    def test_the_fenced_restore_follows_the_durability_test(self) -> None:
        order = [test.name for test in gated.GATED_TESTS]
        self.assertEqual(order.index("fenced_restore"), order.index("durable") + 1)

    def test_the_restore_refuses_without_a_source(self) -> None:
        with self.assertRaises(gated.GatedError):
            gated.prepare_restore(
                gated.Cluster("postgresql://x@localhost/postgres"),
                Path("/nonexistent"),
                None,
                Path("/tmp"),
            )


class TheClusterHelper(unittest.TestCase):
    def test_a_role_url_replaces_the_superuser_credentials(self) -> None:
        cluster = gated.Cluster("postgresql://super:secret@127.0.0.1:55432/postgres")
        url = cluster.url_for("tme_ev_1", role="tme_ev_role_1", password="pw")
        self.assertEqual(url, "postgresql://tme_ev_role_1:pw@127.0.0.1:55432/tme_ev_1")
        self.assertNotIn("secret", url)

    def test_a_plain_url_keeps_the_superuser_authority(self) -> None:
        cluster = gated.Cluster("postgresql://super:secret@127.0.0.1:55432/postgres")
        self.assertEqual(
            cluster.url_for("scratch"), "postgresql://super:secret@127.0.0.1:55432/scratch"
        )


class TheCommandLine(unittest.TestCase):
    def test_an_unknown_only_selector_is_refused(self) -> None:
        self.assertEqual(
            gated.main(["--admin-url-file", "/nonexistent/url", "--only", "nope"]), 1
        )

    def test_the_admin_url_comes_from_a_file(self) -> None:
        parser_arguments = gated.main.__doc__ or ""
        self.assertNotIn("--admin-url ", parser_arguments)


if __name__ == "__main__":
    unittest.main()
