"""One address space, proven rather than assumed.

The Workbench resolves a capture selection by turning a pixel into a target, a
target into a square, and a square into a cell of the compiled member. That last
step is the one everything rests on, and it is the one nobody can see: it is the
claim that a frame row at world position (x, y) **is** master cell (x, y) of the
member the frame was observed on.

The claim is true here because both documents come out of the same compiler run
over the same master, with no offset applied by either. That is a fact about the
current compiler, not a law of nature, so it is asserted — cell by cell, over
both members, with a mutant proving the assertion has teeth.

The recorded frame the ordinary capture route replays is checked against the
same compiled land, because a fixture that drifted from the land would send the
whole capture path to plausible wrong addresses in silence.
"""

from __future__ import annotations

import json
import unittest

from workbench_test_support import (
    REPO_ROOT,
    accepted_projection,
    fixture_route_capture,
    recorded_frame,
)

RUNTIME_PROJECTION = "content/authoring-fixture/generated/world_template.json"

#: The runtime composes a cell's authored terrain stack into one id by joining
#: the classes in authored order. Stated here once so the test asserts a rule
#: rather than a coincidence.
STACK_SEPARATOR = "+"


def runtime_levels() -> dict:
    document = json.loads((REPO_ROOT / RUNTIME_PROJECTION).read_text(encoding="utf-8"))
    realms = document["realms"]
    assert len(realms) == 1, "the fixture land compiles exactly one realm"
    return next(iter(realms.values()))["levels"]


class TheTwoProjectionsShareOneLattice(unittest.TestCase):
    def setUp(self) -> None:
        self.projection = accepted_projection()
        self.levels = runtime_levels()

    def test_the_two_projections_carry_the_same_members_at_the_same_extents(self) -> None:
        self.assertEqual(set(self.levels), set(self.projection.members))
        for name, level in self.levels.items():
            member = self.projection.member(name)
            self.assertEqual((member.width, member.height), (level["width"], level["height"]))
            self.assertEqual(len(member.cells), level["width"] * level["height"])

    def test_a_runtime_cell_is_the_same_address_as_the_logical_cell(self) -> None:
        """No offset, no flip, no transpose — the identity map, cell by cell."""
        for name, level in self.levels.items():
            member = self.projection.member(name)
            with self.subTest(member=name):
                for y in range(level["height"]):
                    for x in range(level["width"]):
                        self.assertEqual(
                            list(level["cells"][y][x]),
                            [entry["class"] for entry in member.terrain((x, y))],
                            f"{name} cell {x},{y}",
                        )

    def test_the_identity_map_is_actually_being_checked(self) -> None:
        """The mutant: shift the comparison by one square and it must fail.

        Without this, a land whose terrain happened to be uniform would let an
        offset through, and the assertion above would be decoration.
        """
        level = self.levels["surface"]
        member = self.projection.member("surface")
        mismatches = sum(
            1
            for y in range(level["height"] - 1)
            for x in range(level["width"] - 1)
            if list(level["cells"][y][x])
            != [entry["class"] for entry in member.terrain((x + 1, y))]
        )
        self.assertGreater(
            mismatches,
            0,
            "a one-square offset must disagree somewhere, or the check proves nothing",
        )


class TheRecordedFrameStandsOnTheCompiledLand(unittest.TestCase):
    """The frame the ordinary capture route replays is a real frame of THIS land."""

    def setUp(self) -> None:
        self.document = recorded_frame()
        self.frame = self.document["frame"]
        self.projection = accepted_projection()
        self.levels = runtime_levels()

    def test_the_frame_declares_where_it_was_observed(self) -> None:
        centre = self.frame["observation_center"]
        self.assertEqual(centre["level"], "surface")
        self.assertEqual(centre["realm"], self.projection.realm_id)
        self.assertIn("provenance", self.document)
        self.assertEqual(self.document["provenance"]["route"], "live")

    def test_every_square_in_the_frame_is_a_square_of_the_compiled_member(self) -> None:
        member = self.projection.member("surface")
        for tile in self.frame["tiles"]:
            square = (int(tile["position"]["x"]), int(tile["position"]["y"]))
            self.assertTrue(member.contains(square), f"the frame shows {square}, the land does not")

    def test_every_squares_terrain_and_passability_are_the_compiled_ones(self) -> None:
        member = self.projection.member("surface")
        level = self.levels["surface"]
        for tile in self.frame["tiles"]:
            x, y = int(tile["position"]["x"]), int(tile["position"]["y"])
            with self.subTest(square=(x, y)):
                self.assertEqual(
                    tile["terrain_id"],
                    STACK_SEPARATOR.join(level["cells"][y][x]),
                    "the frame's terrain id is the compiled stack, joined",
                )
                self.assertEqual(bool(tile["passable"]), member.is_passable((x, y)))

    def test_the_frame_is_the_one_the_tracked_capture_shows(self) -> None:
        """Frame and capture are re-recorded together; a stale pair is a defect."""
        taken = fixture_route_capture()
        self.assertEqual(taken.frame_generation, self.document["frame_generation"])
        self.assertEqual(taken.level, self.frame["observation_center"]["level"])
        self.assertEqual(taken.realm, self.frame["observation_center"]["realm"])
        squares = {
            (int(tile["position"]["x"]), int(tile["position"]["y"]))
            for tile in self.frame["tiles"]
        }
        drawn = {
            (int(record["coordinate"]["x"]), int(record["coordinate"]["y"]))
            for record in taken.targets
            if record["kind"] == "tile"
        }
        self.assertEqual(drawn, squares, "the capture draws exactly the frame's squares")

    def test_the_frames_squares_use_the_lattice_address_form(self) -> None:
        """`tile:<x>:<y>` on the client and cell (x, y) in the master are one form."""
        taken = fixture_route_capture()
        for record in taken.targets:
            if record["kind"] != "tile":
                continue
            with self.subTest(identity=record["identity"]):
                self.assertEqual(
                    record["identity"],
                    f"tile:{record['coordinate']['x']}:{record['coordinate']['y']}",
                )


if __name__ == "__main__":
    unittest.main()
