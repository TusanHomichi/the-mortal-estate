# Inspect Room Fixture Provenance

`inspect_room.json` is clean project-authored prototype content.

It exists to exercise the generic inspect action, local exit statuses, adjacent
actor reporting, and transcript rendering in the local simulation harness.
Names, map layout, text, and numbers are original project prototype choices. The
fixture does not copy raw research material, historical maps, historical
dialogue, historical item names, or source-game data.

The fixture is intentionally small: one player, one adjacent low-threat monster,
and one inspect step are enough to prove the parser, rules event, renderer, and
golden transcript contract.

- Terrain names, movement budgets, terrain costs, and required combat-rule
  numbers are clean project fixture values unless the file explicitly identifies
  a tracked normalized mechanic category. The combat object is marked
  `original_provisional` and is not target-formula evidence.
