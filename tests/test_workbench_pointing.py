"""Acceptance criterion 1 — pointing resolves exactly.

Every assertion here names a region of the **accepted authoring fixture** whose
contents were read off the authored document by hand, not off the resolver.
Click, box, lasso, and paint each get the region they are the natural gesture
for, and the expected covered-cell set and identity set are written out in full
rather than summarized: a test that only counts identities cannot tell a
correct answer from a plausible one.

The fixture's north structure is the anchor. It occupies cells (8,6)-(9,7) on
town ground, its access cell is (8,8), and its façade door is (8,7).
"""

from __future__ import annotations

import unittest

from workbench_test_support import accepted_projection, surface

from workbench.identity import resolve
from workbench.packet import cells_for_gesture

NORTH_FOOTPRINT = [(8, 6), (9, 6), (8, 7), (9, 7)]
STRUCTURE = "structure:surface:fixture_structure_north"
TOWN_GROUND = "terrain:surface:testland_town_ground"
FOOTPRINT_TERRAIN = "terrain:surface:testland_structure_footprint"
BASE_LAYER = "layer:surface:base_terrain"
FOOTPRINT_LAYER = "layer:surface:structure_footprints"


def cells_of(record) -> list[tuple[int, int]]:
    return [(cell["x"], cell["y"]) for cell in record["cells"]]


def identity_map(resolution) -> dict[str, dict]:
    return {record["identity"]: record for record in resolution["semantic"]}


def ordered_identities(resolution) -> list[tuple[str, str]]:
    return [(record["kind"], record["identity"]) for record in resolution["semantic"]]


class Pointing(unittest.TestCase):
    def setUp(self) -> None:
        self.member = surface()

    def resolve_gesture(self, gesture: str, payload: dict):
        cells = cells_for_gesture(self.member, gesture, payload)
        return cells, resolve(self.member, cells)

    # -- click ------------------------------------------------------------

    def test_a_click_on_a_footprint_cell_names_the_structure_that_stands_there(self) -> None:
        cells, resolution = self.resolve_gesture("click", {"cell": {"x": 8, "y": 6}})
        self.assertEqual(cells, [(8, 6)])
        self.assertEqual(cells_of(resolution), [(8, 6)])
        self.assertEqual(
            ordered_identities(resolution),
            [
                ("structure", STRUCTURE),
                ("cell_terrain", FOOTPRINT_TERRAIN),
                ("layer", FOOTPRINT_LAYER),
                ("cell_terrain", TOWN_GROUND),
                ("layer", BASE_LAYER),
            ],
        )
        structure = identity_map(resolution)[STRUCTURE]
        self.assertEqual(structure["detail"]["footprint"],
                         {"x": 8, "y": 6, "width": 2, "height": 2})
        self.assertEqual(structure["detail"]["access_cell"], {"x": 8, "y": 8})
        self.assertEqual(structure["detail"]["facade_door"], {"x": 8, "y": 7})
        self.assertEqual(structure["detail"]["purpose"], "fixture_workshop")
        self.assertFalse(structure["detail"]["access_cell_covered"])
        # One occupant accounting for the whole selection: nothing to ask about.
        self.assertFalse(resolution["ambiguous"])

    def test_a_click_on_plain_ground_names_the_terrain_and_nothing_else(self) -> None:
        _, resolution = self.resolve_gesture("click", {"cell": {"x": 2, "y": 1}})
        self.assertEqual(
            ordered_identities(resolution),
            [("cell_terrain", "terrain:surface:testland_grass"), ("layer", BASE_LAYER)],
        )
        self.assertFalse(resolution["ambiguous"])

    def test_a_click_on_the_transition_marker_names_the_transition_and_its_pair(self) -> None:
        _, resolution = self.resolve_gesture("click", {"cell": {"x": 18, "y": 7}})
        transition = identity_map(resolution)["transition:surface:fixture_descent"]
        self.assertEqual(transition["detail"]["target_member"], "interior")
        self.assertEqual(transition["detail"]["paired_transition"], "fixture_ascent")
        self.assertEqual(transition["detail"]["direction"], "down")
        self.assertEqual(transition["detail"]["access_cell"], {"x": 18, "y": 8})
        self.assertTrue(transition["detail"]["marker_covered"])
        self.assertFalse(transition["detail"]["access_cell_covered"])

    def test_a_click_outside_the_envelope_is_refused(self) -> None:
        with self.assertRaises(Exception) as refused:
            self.resolve_gesture("click", {"cell": {"x": 24, "y": 0}})
        self.assertIn("outside member", str(refused.exception))

    # -- box --------------------------------------------------------------

    def test_a_box_over_the_whole_footprint_covers_exactly_those_four_cells(self) -> None:
        cells, resolution = self.resolve_gesture(
            "box", {"rect": {"x": 8, "y": 6, "width": 2, "height": 2}}
        )
        self.assertEqual(cells, NORTH_FOOTPRINT)
        self.assertEqual(cells_of(resolution), NORTH_FOOTPRINT)
        self.assertEqual(
            ordered_identities(resolution),
            [
                ("structure", STRUCTURE),
                ("cell_terrain", FOOTPRINT_TERRAIN),
                ("layer", FOOTPRINT_LAYER),
                ("cell_terrain", TOWN_GROUND),
                ("layer", BASE_LAYER),
            ],
        )
        identities = identity_map(resolution)
        self.assertEqual(cells_of(identities[STRUCTURE]), NORTH_FOOTPRINT)
        # Four of the structure's five cells: the footprint, not the access cell.
        self.assertEqual(identities[STRUCTURE]["coverage"]["identity_cells"], 5)
        self.assertEqual(identities[STRUCTURE]["coverage"]["identity_coverage"], 0.8)
        self.assertEqual(identities[STRUCTURE]["coverage"]["selection_coverage"], 1.0)
        # The fixture's town ground is a 9x5 block; its footprints stand on it.
        self.assertEqual(identities[TOWN_GROUND]["coverage"]["identity_cells"], 45)
        self.assertEqual(identities[FOOTPRINT_TERRAIN]["coverage"]["identity_cells"], 12)
        # Three cells carry a bridge instead of the deep water it replaced, so
        # three of the 384 cells have no base terrain at all.
        self.assertEqual(identities[BASE_LAYER]["coverage"]["identity_cells"], 381)
        self.assertFalse(resolution["ambiguous"])

    def test_a_box_spanning_two_structures_covers_both_and_the_ground_between(self) -> None:
        cells, resolution = self.resolve_gesture(
            "box", {"rect": {"x": 8, "y": 6, "width": 7, "height": 2}}
        )
        self.assertEqual(len(cells), 14)
        identities = identity_map(resolution)
        self.assertEqual(
            cells_of(identities[STRUCTURE]), NORTH_FOOTPRINT
        )
        self.assertEqual(
            cells_of(identities["structure:surface:fixture_structure_south"]),
            [(13, 6), (14, 6), (13, 7), (14, 7)],
        )
        self.assertTrue(resolution["ambiguous"])

    # -- lasso ------------------------------------------------------------

    def test_a_lasso_and_a_box_over_the_same_region_resolve_identically(self) -> None:
        """Different gestures, one address. The gesture is how you point, not what you pointed at."""
        box_cells, box = self.resolve_gesture(
            "box", {"rect": {"x": 8, "y": 6, "width": 2, "height": 2}}
        )
        lasso_cells, lasso = self.resolve_gesture(
            "lasso",
            {
                "polygon": [
                    {"x": 8.0, "y": 6.0},
                    {"x": 10.0, "y": 6.0},
                    {"x": 10.0, "y": 8.0},
                    {"x": 8.0, "y": 8.0},
                ]
            },
        )
        self.assertEqual(lasso_cells, box_cells)
        self.assertEqual(lasso, box)

    def test_a_lasso_takes_cells_by_their_centres(self) -> None:
        """A right triangle over the town-ground block, checked cell by cell.

        The hypotenuse runs from (11,6) to (8,9), so a centre is inside exactly
        when x + y < 17. Three centres land ON the line — (10.5,6.5), (9.5,7.5),
        (8.5,8.5) — and are excluded. A cell half-covered by the drawn shape is
        not a cell the owner pointed at, and the rule that decides is stated
        here rather than left to whatever the geometry happened to do.
        """
        cells, _ = self.resolve_gesture(
            "lasso",
            {"polygon": [{"x": 8.0, "y": 6.0}, {"x": 11.0, "y": 6.0}, {"x": 8.0, "y": 9.0}]},
        )
        self.assertEqual(cells, [(8, 6), (9, 6), (8, 7)])

    def test_a_lasso_needs_three_points(self) -> None:
        with self.assertRaises(Exception) as refused:
            self.resolve_gesture(
                "lasso", {"polygon": [{"x": 0.0, "y": 0.0}, {"x": 1.0, "y": 1.0}]}
            )
        self.assertIn("three points", str(refused.exception))

    # -- paint ------------------------------------------------------------

    def test_paint_over_the_access_row_names_the_structure_it_opens(self) -> None:
        cells, resolution = self.resolve_gesture(
            "paint",
            {"cells": [{"x": 8, "y": 8}, {"x": 9, "y": 8}, {"x": 10, "y": 8}]},
        )
        self.assertEqual(cells, [(8, 8), (9, 8), (10, 8)])
        identities = identity_map(resolution)
        structure = identities[STRUCTURE]
        self.assertEqual(cells_of(structure), [(8, 8)])
        self.assertTrue(structure["detail"]["access_cell_covered"])
        self.assertEqual(structure["detail"]["footprint_cells_covered"], 0)
        self.assertEqual(structure["coverage"]["selection_coverage"], round(1 / 3, 6))
        self.assertEqual(cells_of(identities[TOWN_GROUND]), [(8, 8), (9, 8), (10, 8)])
        # One occupant that does not account for the whole selection.
        self.assertTrue(resolution["ambiguous"])

    def test_paint_repeating_a_cell_records_it_once(self) -> None:
        cells, _ = self.resolve_gesture(
            "paint",
            {"cells": [{"x": 8, "y": 8}, {"x": 8, "y": 8}, {"x": 9, "y": 8}]},
        )
        self.assertEqual(cells, [(8, 8), (9, 8)])

    # -- routes and landmarks --------------------------------------------

    def test_a_route_cell_reports_membership_over_the_whole_member(self) -> None:
        _, resolution = self.resolve_gesture("click", {"cell": {"x": 12, "y": 12}})
        route = identity_map(resolution)["route:surface"]
        self.assertEqual(route["coverage"]["identity_cells"], 38)
        self.assertEqual(route["detail"]["route_cells_in_member"], 38)

    def test_the_arrival_landmark_resolves_with_its_role(self) -> None:
        _, resolution = self.resolve_gesture("click", {"cell": {"x": 12, "y": 14}})
        landmark = identity_map(resolution)["landmark:surface:fixture_dock_arrival"]
        self.assertEqual(landmark["detail"]["role"], "arrival")
        self.assertEqual(landmark["detail"]["at"], {"x": 12, "y": 14})

    def test_the_interior_member_resolves_in_its_own_frame(self) -> None:
        interior = accepted_projection().member("interior")
        cells = cells_for_gesture(interior, "click", {"cell": {"x": 7, "y": 3}})
        resolution = resolve(interior, cells)
        ascent = identity_map(resolution)["transition:interior:fixture_ascent"]
        self.assertEqual(ascent["detail"]["target_member"], "surface")
        self.assertEqual(ascent["detail"]["direction"], "up")


if __name__ == "__main__":
    unittest.main()
