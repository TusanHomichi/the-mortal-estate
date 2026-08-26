"""Tests for the hostname discipline check, including its P9 mutants."""

from __future__ import annotations

import os
import unittest

from boundary_test_support import BoundaryTestCase, running_as_root

import check_hostnames
from boundary_common import EXIT_FAIL_CLOSED, EXIT_OK, EXIT_VIOLATION

EMPTY_ALLOWLIST = "# no hosts allowed\nplaceholder.invalid  # keeps the file non-empty\n"


class HostnameTestCase(BoundaryTestCase):
    def allowlist(self, content: str = EMPTY_ALLOWLIST) -> str:
        path = self.repo.path / "allowlist.txt"
        path.write_text(content, encoding="utf-8")
        return str(path)

    def check(self, allowlist_content: str = EMPTY_ALLOWLIST):
        return self.run_check(
            check_hostnames.main, "--allowlist", self.allowlist(allowlist_content)
        )


class LabelSets(unittest.TestCase):
    def test_excluded_identifier_labels_are_not_plausible_tlds(self) -> None:
        """The exclusion is applied by construction; this proves it held."""
        self.assertEqual(
            check_hostnames.PLAUSIBLE_TLDS & check_hostnames.EXCLUDED_IDENTIFIER_LABELS,
            frozenset(),
        )

    def test_excluded_labels_are_real_tld_vocabulary(self) -> None:
        """Excluding a label that was never in the set would be a no-op typo."""
        self.assertTrue(
            check_hostnames.EXCLUDED_IDENTIFIER_LABELS
            <= check_hostnames._TLD_VOCABULARY
        )


class ReservedNamesPass(HostnameTestCase):
    def test_reserved_tlds_pass(self) -> None:
        self.repo.write(
            "tests/fixture.gd",
            "\n".join(
                [
                    'const HOST = "server.invalid"',
                    'const ALT = "https://api.test/route"',
                    'const THIRD = "shard.example"',
                    'const FOURTH = "node.localhost"',
                ]
            ),
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_reserved_example_domains_pass(self) -> None:
        self.repo.write(
            "tests/fixture.json",
            '{"a": "example.com", "b": "https://www.example.org/x", "c": "example.net"}',
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_loopback_addresses_pass(self) -> None:
        self.repo.write(
            "config.toml", 'bind = "127.0.0.1:8080"\nany = "0.0.0.0"\nlocal = "localhost:5432"\n'
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_ordinary_filenames_are_not_hosts(self) -> None:
        self.repo.write(
            "notes.md",
            "See run_checks.py, world.tscn, notes.txt, data.json, lib.rs and setup.sh.\n",
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_source_references_and_comments_are_not_hosts(self) -> None:
        """The ambiguous forms: file:line references, decorators, // comments."""
        self.repo.write(
            "notes.md",
            "\n".join(
                [
                    "Failure at tools/run_checks.py:31 and at world.tscn:12.",
                    "@unittest.skipIf(condition)",
                    "// notes.md and //draft.md are comments, not authorities",
                ]
            ),
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_rust_field_access_is_not_a_host(self) -> None:
        """Tier 3's label set excludes code-identifier vocabulary.

        The ported crates produced 894 of these and zero true hits, which is
        what a false-positive class looks like before it teaches people to
        ignore the check.
        """
        self.repo.write(
            "crates/tme-rules/src/engine/navigation.rs",
            "\n".join(
                [
                    "let region = engine.world.region(id);",
                    "let label = item.name.clone();",
                    "if actor.location.site == origin.site {",
                    "    let span = edge.at.max(left.at).min(right.at);",
                    "    self.live = span;",
                    "}",
                    "let detail = self.info.summary();",
                ]
            ),
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_allowlisted_host_passes(self) -> None:
        self.repo.write("README.md", "See <https://www.gnu.org/licenses/>.\n")
        code, output = self.check("www.gnu.org  # license URL\n")
        self.assertEqual(code, EXIT_OK, output)

    def test_binary_files_are_skipped(self) -> None:
        self.repo.write_bytes("blob.bin", b"\x00 mutant-host.com \x00")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)


class Mutants(HostnameTestCase):
    """P9: deliberate violations the check must kill."""

    def test_mutant_live_external_hostname_is_killed(self) -> None:
        self.repo.write(
            "client/tests/test_endpoint_resolution.gd",
            'const DEFAULT_HOST = "mutant-host.com"\n',
        )
        self.repo.track("client/tests/test_endpoint_resolution.gd")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("non-reserved hostname", output)

    def test_mutant_url_with_unusual_tld_is_killed(self) -> None:
        self.repo.write("docs/notes.md", "Fetch it from https://mutant-url.sh/data\n")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_mutant_host_with_port_is_killed(self) -> None:
        self.repo.write("config.toml", 'upstream = "mutant-port.zz:5432"\n')
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)

    def test_mutant_public_ipv4_address_is_killed(self) -> None:
        self.repo.write("deploy/hosts.txt", "primary 45.77.12.9\n")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("non-loopback IPv4", output)

    def test_mutant_bare_host_on_a_plausible_tld_is_killed(self) -> None:
        """Tier 3 still fires after the identifier-vocabulary trim.

        Trimming labels narrows tier 3, so tier 3 needs a standing mutant of
        its own: a bare host, in source, under a label that survived.
        """
        self.repo.write(
            "crates/tme-server/src/config.rs",
            'const REGISTRY: &str = "records.mutant-estate.org";\n',
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("records.mutant-estate.org", output)

    def test_mutant_excluded_label_still_dies_in_url_form(self) -> None:
        """What tier 3 gave up, tier 1 still catches.

        `.world` is excluded from tier 3 because `engine.world` is field
        access. A real host under `.world` is still a violation, and this is
        the proof the trim narrowed the tier without opening a hole.
        """
        self.repo.write(
            "docs/notes.md", "Mirrored at https://archive.mutant-estate.world/x\n"
        )
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("archive.mutant-estate.world", output)

    def test_mutant_survives_a_narrower_allowlist(self) -> None:
        """Allowlisting one host does not excuse a different one."""
        self.repo.write("docs/notes.md", "hosts: www.gnu.org and mutant-second.com\n")
        code, output = self.check("www.gnu.org  # license URL\n")
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("mutant-second.com", output)
        self.assertNotIn("www.gnu.org", output)


class FailClosed(HostnameTestCase):
    def test_missing_allowlist_fails_closed(self) -> None:
        self.repo.write("README.md", "clean\n")
        code, output = self.run_check(
            check_hostnames.main,
            "--allowlist",
            str(self.repo.path / "absent.txt"),
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("missing", output)

    def test_entry_less_allowlist_fails_closed(self) -> None:
        self.repo.write("README.md", "clean\n")
        code, output = self.check("# nothing but a comment\n")
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("no entries", output)

    @unittest.skipIf(running_as_root(), "uid 0 ignores the permission bits")
    def test_unreadable_allowlist_fails_closed(self) -> None:
        path = self.repo.path / "allowlist.txt"
        path.write_text("www.gnu.org  # license URL\n", encoding="utf-8")
        os.chmod(path, 0o000)
        self.addCleanup(os.chmod, path, 0o644)
        code, output = self.run_check(
            check_hostnames.main, "--allowlist", str(path)
        )
        self.assertEqual(code, EXIT_FAIL_CLOSED, output)
        self.assertIn("unreadable", output)


if __name__ == "__main__":
    unittest.main()
