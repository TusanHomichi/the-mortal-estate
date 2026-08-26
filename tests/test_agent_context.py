"""The subject router, and the drift it exists to prevent.

The router's whole value is that the routing is **derived**. So the tests are
mostly about the ways derivation can be wrong: a document with no front matter,
a document nobody routes to, a route matching nothing, and a contract table that
has drifted from `docs/`. Each of those is a planted mutant against a temporary
tree; none exists in this one.
"""

from __future__ import annotations

import shutil
import tempfile
import unittest
from pathlib import Path

from verification_test_support import REPO_ROOT

import agent_context


class FrontMatter(unittest.TestCase):
    def test_scalars_and_block_lists_both_parse(self) -> None:
        parsed = agent_context.read_front_matter(
            "---\nsummary: a thing\nroutes:\n  - a/**\n  - b.py\n---\n\n# Body\n"
        )
        self.assertEqual(parsed["summary"], "a thing")
        self.assertEqual(parsed["routes"], ["a/**", "b.py"])

    def test_a_document_without_front_matter_reads_as_none(self) -> None:
        self.assertIsNone(agent_context.read_front_matter("# Just a heading\n"))

    def test_unterminated_front_matter_fails_closed(self) -> None:
        with self.assertRaises(agent_context.RoutingError):
            agent_context.read_front_matter("---\nsummary: x\n\n# Body\n")

    def test_a_list_entry_before_any_key_fails_closed(self) -> None:
        with self.assertRaises(agent_context.RoutingError):
            agent_context.read_front_matter("---\n  - orphan\n---\n")


class Globs(unittest.TestCase):
    def test_double_star_crosses_directories(self) -> None:
        self.assertTrue(agent_context._glob("crates/tme-server/src/store/mod.rs", "crates/tme-server/**"))

    def test_single_star_does_not_cross_directories(self) -> None:
        self.assertTrue(agent_context._glob("tools/check_hostnames.py", "tools/check_*.py"))
        self.assertFalse(agent_context._glob("tools/nested/check_x.py", "tools/check_*.py"))

    def test_an_exact_path_matches_itself_only(self) -> None:
        self.assertTrue(agent_context._glob(".gitignore", ".gitignore"))
        self.assertFalse(agent_context._glob("docs/.gitignore", ".gitignore"))


class ThisTree(unittest.TestCase):
    def setUp(self) -> None:
        self.documents = agent_context.load()

    def test_the_routing_validates(self) -> None:
        self.assertEqual(agent_context.validate(), [])

    def test_every_document_under_docs_is_routable(self) -> None:
        carried = {path.name for path in (REPO_ROOT / "docs").glob("*.md")}
        self.assertEqual({Path(item.path).name for item in self.documents}, carried)

    def test_a_server_path_reaches_the_server_notes_first(self) -> None:
        selected = agent_context.read_first(self.documents, "crates/tme-server/src/postgres.rs")
        self.assertEqual(selected[0].subject, "server-notes")

    def test_a_client_path_reaches_the_client_documents(self) -> None:
        subjects = {
            item.subject
            for item in agent_context.read_first(self.documents, "client/presentation/grid_world_view.gd")
        }
        self.assertIn("client-architecture", subjects)
        self.assertIn("presentation-direction", subjects)

    def test_a_workbench_path_reaches_the_workbench_and_the_working_root(self) -> None:
        subjects = {
            item.subject
            for item in agent_context.read_first(self.documents, "tools/workbench_prune.py")
        }
        self.assertIn("workbench-v0", subjects)
        self.assertIn("working-root-policy", subjects)

    def test_the_always_documents_are_always_included(self) -> None:
        for path in ("crates/tme-rules/src/lib.rs", "client/project.godot", "nothing/at/all.bin"):
            subjects = {item.subject for item in agent_context.read_first(self.documents, path)}
            self.assertTrue({"boundary-map", "agent-workflow", "settled-conclusions"} <= subjects, path)

    def test_an_unrouted_path_still_returns_the_standing_documents(self) -> None:
        selected = agent_context.read_first(self.documents, "nothing/at/all.bin")
        self.assertTrue(all(item.always for item in selected))


class Mutants(unittest.TestCase):
    """P9: each is a way the routing could silently stop being true."""

    def setUp(self) -> None:
        self.root = Path(tempfile.mkdtemp(prefix="tme-routing-")).resolve()
        self.addCleanup(shutil.rmtree, self.root, True)
        (self.root / "docs").mkdir()
        (self.root / "tools").mkdir()
        self.write_contract(["docs/one.md"])
        self.write_doc("one.md", routes=["tools/**"])

    def write_doc(self, name: str, *, routes=(), always=False, front=True) -> None:
        body = "# A document\n"
        if not front:
            (self.root / "docs" / name).write_text(body, encoding="utf-8")
            return
        lines = [
            "---",
            "last_updated: 2026-08-20",
            "status: test",
            "summary: a test document",
        ]
        if always:
            lines.append("always: true")
        if routes:
            lines.append("routes:")
            lines.extend(f"  - {route}" for route in routes)
        lines.append("---")
        (self.root / "docs" / name).write_text("\n".join(lines) + "\n\n" + body, encoding="utf-8")

    def write_contract(self, targets: list[str]) -> None:
        rows = "\n".join(f"| [x]({target}) | when |" for target in targets)
        (self.root / "AGENTS.md").write_text(
            f"# Agent guide\n\n## Read first\n\n| Start here | When |\n| --- | --- |\n{rows}\n\n## Next\n",
            encoding="utf-8",
        )

    def test_mutant_document_with_no_front_matter_fails_closed(self) -> None:
        self.write_doc("two.md", front=False)
        with self.assertRaises(agent_context.RoutingError) as raised:
            agent_context.load(self.root)
        self.assertIn("carries no front matter", str(raised.exception))

    def test_mutant_document_missing_a_required_key_fails_closed(self) -> None:
        (self.root / "docs" / "two.md").write_text(
            "---\nsummary: only this\n---\n\n# Body\n", encoding="utf-8"
        )
        with self.assertRaises(agent_context.RoutingError):
            agent_context.load(self.root)

    def test_mutant_document_absent_from_the_contract_is_killed(self) -> None:
        self.write_doc("two.md", routes=["tools/**"])
        problems = agent_context.validate(self.root)
        self.assertIn("does not list docs/two.md", " ".join(problems))

    def test_mutant_contract_row_for_a_deleted_document_is_killed(self) -> None:
        self.write_contract(["docs/one.md", "docs/deleted.md"])
        problems = agent_context.validate(self.root)
        self.assertIn("which docs/ does not carry", " ".join(problems))

    def test_mutant_route_matching_nothing_is_killed(self) -> None:
        self.write_doc("one.md", routes=["crates/that-does-not-exist/**"])
        problems = agent_context.validate(self.root)
        self.assertIn("matches nothing in this tree", " ".join(problems))

    def test_mutant_document_nobody_can_reach_is_killed(self) -> None:
        self.write_doc("one.md")
        problems = agent_context.validate(self.root)
        self.assertIn("no task will be sent to it", " ".join(problems))

    def test_a_correct_tree_validates(self) -> None:
        self.assertEqual(agent_context.validate(self.root), [])


class TheCommandLine(unittest.TestCase):
    def test_validate_exits_zero_on_this_tree(self) -> None:
        self.assertEqual(agent_context.main(["--validate"]), 0)

    def test_an_unknown_subject_is_an_error(self) -> None:
        self.assertEqual(agent_context.main(["--subject", "no-such-document"]), 1)

    def test_a_known_subject_is_printed(self) -> None:
        self.assertEqual(agent_context.main(["--subject", "boundary-map"]), 0)

    def test_listing_works(self) -> None:
        self.assertEqual(agent_context.main(["--list"]), 0)


if __name__ == "__main__":
    unittest.main()
