# First Room Provenance

`first_room.json` is clean project content.

It is not copied research output, not a converted map, not extracted data, and
not a placeholder asset. It exists to give the future rules core and local
simulation harness a small, reviewable content contract.

## Evidence Use

The local research digest informed only broad first-prototype categories:

- a tiny enclosed room is enough to test blocked and walkable terrain
- one player and one weak monster are enough to test movement, same-hex
  engagement, and ordinary melee combat
- low single-digit weapon-add values are useful starter tuning seeds
- qualitative damage labels should be preserved as a future rules question
- spell names can exist as stubs before spell mechanics are designed

## Original Authorship

The fixture authors these parts from scratch:

- map layout
- actor names
- item names
- spell names
- prose
- exact stat values
- script sequence, including explicit attack after engagement
- terrain names, movement budgets, terrain costs, and required combat-rule
  numbers are clean project fixture values unless the file explicitly identifies
  a tracked normalized mechanic category
- the combat-rule object is visibly `original_provisional`; it preserves the
  deterministic prototype and does not claim a recovered target formula

The current numbers are deliberately conservative local-sim seeds. They are not
claims about historical formulas or tables.

## Review Notes

Future rules work should treat this fixture as an executable contract, not as a
complete content model. If research evidence later changes a mechanic, update
the rules and this fixture through the normal review/normalization boundary.
