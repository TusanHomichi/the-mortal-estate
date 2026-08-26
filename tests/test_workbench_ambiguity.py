"""Acceptance criterion 5 — ambiguity is data, not a guess.

Overlapping identities all reach the packet, ranked, with the flag set, and no
consumer picks one. The flag's rule is stated in `workbench.identity` and each
of its three clauses is exercised here separately, because a rule with an
unexercised clause is a rule with an assumption in it.

The negative direction matters as much as the positive: a flag that is always
true is a flag nobody reads, so the unambiguous cases are asserted too.
"""

from __future__ import annotations

import unittest

from workbench_test_support import accepted_projection, surface

from workbench.identity import OCCUPANT_KINDS, resolve
from workbench.packet import cells_for_gesture


def occupants(resolution) -> list[str]:
    return [
        record["identity"]
        for record in resolution["semantic"]
        if record["kind"] in OCCUPANT_KINDS
    ]


class AmbiguityIsData(unittest.TestCase):
    def setUp(self) -> None:
        self.member = surface()

    def resolve_cells(self, cells):
        return resolve(self.member, cells)

    def test_two_occupants_over_one_cell_are_both_emitted_and_flagged(self) -> None:
        """The arrival landmark stands on a route cell. Both are true at once."""
        resolution = self.resolve_cells([(12, 14)])
        self.assertEqual(
            sorted(occupants(resolution)),
            ["landmark:surface:fixture_dock_arrival", "route:surface"],
        )
        self.assertTrue(resolution["ambiguous"])

    def test_an_ambiguous_selection_carries_every_identity_as_a_candidate(self) -> None:
        resolution = self.resolve_cells([(12, 14)])
        self.assertEqual(resolution["candidates"], resolution["semantic"])
        self.assertGreater(len(resolution["candidates"]), 1)

    def test_an_unambiguous_selection_carries_no_candidate_list(self) -> None:
        resolution = self.resolve_cells([(2, 1)])
        self.assertFalse(resolution["ambiguous"])
        self.assertEqual(resolution["candidates"], [])

    def test_one_occupant_that_does_not_explain_the_selection_is_ambiguous(self) -> None:
        """Clause two: the structure is there, but most of what was pointed at is not it."""
        resolution = self.resolve_cells([(8, 8), (9, 8), (10, 8)])
        self.assertEqual(occupants(resolution), ["structure:surface:fixture_structure_north"])
        self.assertTrue(resolution["ambiguous"])

    def test_one_occupant_that_explains_the_whole_selection_is_not_ambiguous(self) -> None:
        resolution = self.resolve_cells([(8, 6), (9, 6), (8, 7), (9, 7)])
        self.assertEqual(occupants(resolution), ["structure:surface:fixture_structure_north"])
        self.assertFalse(resolution["ambiguous"])

    def test_two_terrains_and_no_occupant_are_ambiguous(self) -> None:
        """Clause three: grass and forest, with nothing standing on either."""
        resolution = self.resolve_cells([(9, 3), (6, 3)])
        self.assertEqual(occupants(resolution), [])
        self.assertEqual(
            sorted(
                record["identity"]
                for record in resolution["semantic"]
                if record["kind"] == "cell_terrain"
            ),
            ["terrain:surface:testland_forest", "terrain:surface:testland_grass"],
        )
        self.assertTrue(resolution["ambiguous"])

    def test_one_terrain_and_no_occupant_is_not_ambiguous(self) -> None:
        resolution = self.resolve_cells([(6, 3), (7, 3)])
        self.assertFalse(resolution["ambiguous"])

    def test_ranking_orders_by_share_of_the_selection_then_by_kind(self) -> None:
        """Ranking is presentation. It must be total, stable, and never a verdict."""
        resolution = self.resolve_cells([(8, 6), (9, 6), (8, 7), (9, 7), (8, 8)])
        ranks = [record["rank"] for record in resolution["semantic"]]
        self.assertEqual(ranks, list(range(1, len(ranks) + 1)))
        coverages = [
            record["coverage"]["selection_coverage"] for record in resolution["semantic"]
        ]
        self.assertEqual(coverages, sorted(coverages, reverse=True))
        # The structure covers all five cells; it leads, and the flag is still
        # clear because it explains the whole selection.
        self.assertEqual(
            resolution["semantic"][0]["identity"], "structure:surface:fixture_structure_north"
        )
        self.assertFalse(resolution["ambiguous"])

    def test_resolution_is_stable_across_runs(self) -> None:
        cells = [(8, 6), (12, 14), (18, 7)]
        self.assertEqual(self.resolve_cells(cells), self.resolve_cells(list(reversed(cells))))

    def test_a_selection_spanning_three_occupants_emits_all_three(self) -> None:
        member = accepted_projection().member("surface")
        cells = cells_for_gesture(
            member, "box", {"rect": {"x": 8, "y": 6, "width": 11, "height": 3}}
        )
        resolution = resolve(member, cells)
        self.assertEqual(
            sorted(occupants(resolution)),
            [
                "route:surface",
                "structure:surface:fixture_structure_north",
                "structure:surface:fixture_structure_south",
                "transition:surface:fixture_descent",
            ],
        )
        self.assertTrue(resolution["ambiguous"])


if __name__ == "__main__":
    unittest.main()
