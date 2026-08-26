"""Tests for the markdown link check, including its P9 mutants.

The two mutants issue #9 names — a dead link and a dead anchor — are the first
two cases in `Mutants`. Both are planted in a temporary repository; neither
ever exists in this tree.
"""

from __future__ import annotations

import unittest

from boundary_test_support import BoundaryTestCase

import check_markdown_links
from boundary_common import EXIT_OK, EXIT_VIOLATION


class LinkCheckTestCase(BoundaryTestCase):
    def check(self):
        return self.run_check(check_markdown_links.main)

    def compliant(self) -> None:
        self.repo.write("docs/target.md", "# The Target\n\n## A Second Heading\n")
        self.repo.write(
            "README.md",
            "See [the target](docs/target.md) and [its heading](docs/target.md#a-second-heading).\n",
        )
        self.repo.track("docs/target.md", "README.md")


class CompliantTree(LinkCheckTestCase):
    def test_resolving_links_and_anchors_pass(self) -> None:
        self.compliant()
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)
        self.assertIn("markdown-links: OK", output)

    def test_external_links_are_out_of_scope(self) -> None:
        self.repo.write(
            "README.md",
            "[web](https://example.invalid/nothing) [mail](mailto:nobody@example.invalid)\n",
        )
        self.repo.track("README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_a_link_inside_a_fence_is_not_a_link(self) -> None:
        self.repo.write("README.md", "```\n[dead](docs/gone.md)\n```\n")
        self.repo.track("README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_a_directory_target_is_allowed_when_it_holds_a_carried_file(self) -> None:
        self.repo.write("docs/target.md", "# Target\n")
        self.repo.write("README.md", "See [the docs](docs/).\n")
        self.repo.track("docs/target.md", "README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_a_repeated_heading_gets_its_numbered_anchor(self) -> None:
        self.repo.write("docs/target.md", "# Rulings\n\n# Rulings\n")
        self.repo.write("README.md", "[second](docs/target.md#rulings-1)\n")
        self.repo.track("docs/target.md", "README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)

    def test_an_explicit_html_anchor_resolves(self) -> None:
        self.repo.write("docs/target.md", '<a id="hand-written"></a>\n\n# Heading\n')
        self.repo.write("README.md", "[anchor](docs/target.md#hand-written)\n")
        self.repo.track("docs/target.md", "README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_OK, output)


class Mutants(LinkCheckTestCase):
    """P9: deliberate defects the check must kill."""

    def test_mutant_dead_link_is_killed(self) -> None:
        self.repo.write("README.md", "See [the plan](docs/no-such-plan.md).\n")
        self.repo.track("README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("link target does not exist: docs/no-such-plan.md", output)

    def test_mutant_dead_anchor_is_killed(self) -> None:
        self.repo.write("docs/target.md", "# The Target\n")
        self.repo.write("README.md", "See [a section](docs/target.md#no-such-section).\n")
        self.repo.track("docs/target.md", "README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("no such anchor: docs/target.md#no-such-section", output)

    def test_mutant_link_to_an_ignored_file_is_killed(self) -> None:
        self.repo.write(".gitignore", "secret/\n")
        self.repo.write("secret/notes.md", "# Private\n")
        self.repo.write("README.md", "See [notes](secret/notes.md).\n")
        self.repo.track(".gitignore", "README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("not carried by git: secret/notes.md", output)

    def test_mutant_link_escaping_the_repository_is_killed(self) -> None:
        self.repo.write("docs/README.md", "See [outside](../../elsewhere.md).\n")
        self.repo.track("docs/README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("escapes the repository", output)

    def test_mutant_absolute_link_is_killed(self) -> None:
        self.repo.write("README.md", "See [root](/etc/passwd).\n")
        self.repo.track("README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("absolute link target", output)

    def test_mutant_dead_reference_definition_is_killed(self) -> None:
        self.repo.write("README.md", "See [the plan][plan].\n\n[plan]: docs/gone.md\n")
        self.repo.track("README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("docs/gone.md", output)

    def test_mutant_dead_same_file_anchor_is_killed(self) -> None:
        self.repo.write("README.md", "# Here\n\nSee [below](#nowhere).\n")
        self.repo.track("README.md")
        code, output = self.check()
        self.assertEqual(code, EXIT_VIOLATION, output)
        self.assertIn("no such anchor: #nowhere", output)


class SlugRule(unittest.TestCase):
    def test_each_space_becomes_one_hyphen(self) -> None:
        # "observer / debug" leaves two spaces once the slash is stripped, and
        # GitHub emits two hyphens. Collapsing them would reject a live link.
        self.assertEqual(
            check_markdown_links.slug("1.14 The observer / debug projection split"),
            "114-the-observer--debug-projection-split",
        )

    def test_code_spans_and_links_are_unwrapped(self) -> None:
        self.assertEqual(check_markdown_links.slug("The `pulse` value"), "the-pulse-value")
        self.assertEqual(check_markdown_links.slug("See [D5](x.md)"), "see-d5")


class ThisRepository(unittest.TestCase):
    def test_every_link_in_this_tree_resolves(self) -> None:
        """The check, against the tree it guards. This is the standing claim."""
        self.assertEqual(check_markdown_links.main([]), EXIT_OK)


if __name__ == "__main__":
    unittest.main()
