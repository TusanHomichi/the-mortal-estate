---
last_updated: 2026-09-07
revision: 1
status: Owner-directed temple residents, authored circuits and actor-bound services implemented; native verification recorded in the town execution record.
public_safe: true
summary: Resident movement, service position ownership, observation identity, persistence and temple-specific direction.
routes:
  - crates/tme-rules/src/model/npcs.rs
  - crates/tme-rules/src/model/services.rs
  - crates/tme-rules/src/content/npcs.rs
  - crates/tme-rules/src/engine/npcs.rs
  - crates/tme-rules/src/engine/npcs/**
  - crates/tme-rules/src/engine/services.rs
---

# Town residents

The owner requested an older female balm seller beside rear shelves and young,
friendly, tired Tomas wandering the temple. This is direction for the rebuilt
town under the [historical baseline](gameplay-baseline.md), not a claim that
these residents or their circuit reproduce a historical NPC routine.
[Presentation direction](presentation-direction.md#placeholder-town-interiors)
owns their appearance; [browser client](browser-client.md#temple-resident-interaction-direction)
owns the direct interaction surface. The [town execution record](plans/2026-09-06-town-buildout.md)
owns this implementation's evidence.

## Movement and attention

`NpcDef.patrol` explicitly declares a closed circuit of cardinally adjacent,
passable cells in the resident's home area. An empty circuit means no patrol.
The seed validator refuses a one-cell circuit, disconnected steps, blocked
ground, more than 64 cells, and a circuit missing the initial resident square.
The rules own the current circuit cursor and every move. Existing combat
priority and escort following precede an idle patrol; ordinary movement
legality still applies. A resident outside its home area does not patrol there.

The existing `follow_cadence_units` supplies the resident's automatic travel
cadence, including patrol. Tomas's authored value is three units, currently nine
seconds under [D5](boundary-map.md#21-authoritative-individual-deadlines-d5).
This is a provisional presentation cadence for the requested wandering,
independent of the standard player movement deadline. A living player on the
same or cardinally adjacent square holds the resident's attention at each
opportunity. This pause grants no additional service range. Maude remains by her
supplies. Their exact cells and Tomas's circuit have one owner:
`content/lands/first-expedition/simulation_seed.json`.

The ordinary server scheduler advances these opportunities without player
commands. Checkpoints retain the circuit, cursor and deadline. Hydration refuses
invalid cursors, unknown service providers and circuits crossing blocked ground.

## Service position and identity

A service instance has exactly one `placement`: `fixed` with a world location,
or `actor` with a known NPC identity. An attached service has no second stored
coordinate. `engine/services.rs` resolves its current location from its living
provider. Maude's shop and Tomas's restoration service use actor placements.
Definitions still own prices, capabilities, eligibility and effects.

Discovery projects the explicit provider `actor_id` with each local service;
fixed services carry an explicit null. Protocol minor 10 requires that field and
refuses the retired minor. The client never joins a shop to a person by their
display name or by assuming that actor and service IDs are equal.

Opening a resident reveals the current projected offers and sends no command.
A purchase uses the existing assessed transaction and is revalidated at commit.
Moving away, losing funds or losing the provider cannot preserve an old usable
offer. Existing same-square service requirements remain in force. Every service gate
compares realm, area and coordinate together; equal area names in different
realms cannot grant access.

## Proof

`engine/npcs/tests.rs` exercises the authored temple, continuing patrol after
checkpoint hydration, approaching-player attention, moving service discovery,
dead-provider refusal and malformed source refusal. The wire corpus includes a
missing-provider-field rejection consumed by both native Rust and browser WASM.
Browser interaction tests prove identity-based grouping and unavailable offers.
The execution record adds the actual three-engine right-click and purchase loop.
