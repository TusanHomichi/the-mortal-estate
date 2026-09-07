---
last_updated: 2026-09-06
revision: 2
status: Initial recovery audit complete; its proposed amendment was subsequently accepted through the historical gameplay baseline.
public_safe: true
summary: Surviving gameplay implementation, missing design continuity, and the trainer-layout correction.
---

# Design continuity audit

The owner identified a loss of gameplay and class-design context during the
successor cutover. The intended development approach was to adapt a familiar
class and mechanics baseline, including changes to names and behavior. Generic
trainer stations in the town candidate exposed the missing connection between
that intent and current content work.

This is a historical and implementation inventory. Its proposed amendment was
subsequently superseded by the owner-directed [gameplay baseline](../gameplay-baseline.md).
Read its adoption proposals below as the initial audit's historical status.
[D2](../boundary-map.md#what-an-authored-seam-does-and-does-not-settle) and the
[public boundary](../public-boundary-policy.md) remain the current owners of
adoption and sourcing. The audit introduces no historical payload into the tree.

## What remains available

Inspection of the current checkout establishes these surviving implementations
and conformance examples. Presence does not establish final product acceptance,
complete behavior, or availability through the browser client.

| Area | Current evidence | Practical consequence |
| --- | --- | --- |
| Class identities | `content/test-corpus/catalogs/prototype_catalog_v6.json` references fighter, knight, martial_artist, thief, wizard and thaumaturge. | Six identities survive; the candidate's four station labels do not define the roster. |
| Class-specific behavior | `crates/tme-rules/src/engine/professions.rs`, `promotion.rs`, and profession/promotion scenarios. | Hiding, unarmed defense and promotion have existing implementation to assess. |
| Skills and training | `crates/tme-rules/src/engine/skills.rs`, `training.rs`, and `tests/cases/training_transactions.rs`. | Practice, learning rates and training transactions have existing owners and proof cases. |
| Magic | `crates/tme-rules/src/engine/spellcasting/`, `spell_learning.rs`; 73 catalog spell definitions. | Spell acquisition and casting need a product design reconciliation, not an assumption of absence. |
| Inventory and services | 76 catalog item definitions, three bank definitions, three locker-vault definitions, and `crates/tme-rules/src/engine/storage/`. | The visual town services can eventually connect to existing rule domains. Counts are definitions, not distinct finished products or deployed services. |
| Combat, resources, death and law | `crates/tme-rules/src/engine/combat.rs`, `resources.rs`, `death.rs`, `restoration.rs`, `social.rs`. | Review surviving behavior against later decisions before scheduling replacement work. |

The [test-corpus provenance owner](../test-corpus-provenance.md) explains why
these examples are conformance content rather than accepted world content.
Current authoritative timing is the later
[D5 ruling](../boundary-map.md#21-authoritative-individual-deadlines-d5).

## Finding and immediate correction

The carried architecture and tests remain substantial. A maintained gameplay
design inventory connecting the intended class roles, progression, trainers and
town services is missing. Treating that documentation gap as permission to
invent unrelated trainer categories was an implementation-planning error.

The [town buildout](2026-09-06-town-buildout.md) has four visual stations labelled
ARMS, STUDY, FOCUS and RANGE. Their labels and count are provisional spatial
studies. They neither replace the surviving class identities nor establish a
four-class design. Trainer assignment and layout need reconciliation before
further class-specific content work; no one-trainer-per-class rule is inferred.

**Owner:** gameplay design, with town authoring responsible for the resulting
spatial changes. **Required proof:** an explicit class/role and training-track
matrix, dispositions for previous decisions, later-ruling conflict checks, and
an authored mapping from actual services to their town locations. Implementation
proof must then exercise those services through the real client path.

## Recovery status and proposed next slice

An external, digest-recorded recovery packet preserves 31 historical documents
and content files, including the original migration rulings, charters, migration
inventory, class/progression evaluations and catalog. Comparison finds the same
six class identities and the same spell, item, bank and locker definition counts
in the historical and current catalogs. This is structural evidence; it is not
a byte-equivalence or behavior-equivalence claim. Source material and private
provenance remain outside the checkout and are not build dependencies.

The recovery also distinguishes previously authored design direction from
source observations and provisional tuning. Historical trainer interaction
direction was more specific than the current visual placeholder work. It needs
an explicit adoption disposition under current rulings, rather than silent
promotion or omission. The initial pass does not claim to exhaust historical
conversations, issues, or private research records.

The proposed next slice is a class-and-training design reconciliation: recover
the familiar roles as a reviewable adaptation baseline, identify what is retained,
renamed or changed, and map trainers to the resulting design. Broad baseline
adoption requires an explicit amendment to D2; this audit does not infer that
amendment from the owner's retrospective preference. Preserve later accepted
timing, geographic and presentation rulings during that decision.

Validation for this documentation slice uses the selected documentation and
boundary lanes. No gameplay code changes or new runtime test verdicts are claimed.
