# Combat Labels Fixture Provenance

`combat_labels.json` is clean project-authored prototype content.

It exists to exercise repeated deterministic same-hex melee labels in the local
simulation harness through explicit attack script steps. Names, map layout,
text, and numbers are original project prototype choices. The fixture does not
copy raw research material, historical maps, historical dialogue, historical
item names, or source-game data.

The fixture is intentionally artificial: equal HP and simple stats make the
seeded attack sequence produce a useful spread of labels for tests.

- Terrain names, movement budgets, terrain costs, and required combat-rule
  numbers are clean project fixture values unless the file explicitly identifies
  a tracked normalized mechanic category.

Equipment and item effects are active runtime mechanics. This focused fixture
keeps `items` empty so its transcript isolates the required
`original_provisional` combat tuning and damage-label thresholds from equipment
contributions.
