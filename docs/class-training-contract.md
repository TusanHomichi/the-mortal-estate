---
last_updated: 2026-09-09
revision: 9
status: Five authored creation profiles and bounded town instruction are implemented; typed critique delivered; historical numerical reconciliation and later class display naming remain open; Martial Artist is retained for now.
public_safe: true
summary: Recovered creation bounds, nationality presentation deferral, durable creation, instruction and provisional values.
routes:
  - crates/tme-rules/src/engine/training.rs
  - crates/tme-rules/src/engine/training/**
  - crates/tme-rules/src/engine/skills.rs
  - crates/tme-rules/src/engine/professions.rs
  - crates/tme-rules/src/engine/promotion.rs
  - crates/tme-rules/src/engine/creation.rs
  - crates/tme-rules/src/engine/creation/**
  - crates/tme-rules/src/content/creation.rs
  - crates/tme-rules/src/engine/action_context/service_discovery.rs
  - web/src/play/**
---

# Classes and training

This Canonical document owns the recovered class/teacher relationships and
training interaction contract. The [gameplay baseline](gameplay-baseline.md)
owns fidelity and version selection. The [boundary map](boundary-map.md#111-one-mutation-boundary-per-actor-state-family)
owns implementation seams. Public labels below are existing mechanical role
labels; they do not settle final names or NPC dialogue.
The owner directed a display-name discussion as each class's reusable art begins
(September 8), starting with Martial Artist. Existing class labels remain working
names. The owner explicitly retained Martial Artist for now after discussing
alternatives. A display-name decision does not itself alter
class mechanics, content IDs, creation profiles or teacher relationships.


## Class and teacher relationships

There are five starting classes. Knight is a Fighter promotion, available from
experience level eight, rather than a sixth creation choice. Basic promotion
requires no gold; its complete eligibility and ring transaction need separate
proof before implementation is described as corresponding exactly.

| Class | Distinctive capabilities | Teaching relationship in the rebuilt town |
| --- | --- | --- |
| Fighter | Physical weapons; route to Knight promotion | Shared weapons instruction in the practice court. |
| Martial Artist | Unarmed fighting, blocking, kick and jumpkick | Hand and weapons instruction in the practice court; advanced teachers remain separate content. |
| Thief | Concealment, thievery, weapons and its own magic lane | Guild instruction combines thievery and Thief magic; guild weapons instruction may restrict admission to Thieves. |
| Wizard | Wizard magic and supporting weapon skills | Wizard study provides its own magic training and spell teaching; physical instruction remains available elsewhere. |
| Thaumaturge | Weapons and its own priestly magic lane | Temple instruction provides its magic training and spell teaching, alongside the separately owned balm and resurrection services. |
| Knight (promotion) | Fighter capabilities plus ring-mediated powers | Promotion belongs to the hazardous surface journey beyond town; do not add a sixth starter or move the promotion into the practice court. |

A teacher can serve several classes and teach several tracks. Access, teaching
knowledge and permitted skill range are separate conditions. Magic instruction
requires teacher and pupil to share the relevant occupation. Four generic
candidate signs do not establish four classes, four exclusive teachers, or four
mutually exclusive skill categories.

The reconstructed historical relationships bind these service assignments; the
new town's internal placement is authored anew under the
[surface brief](plans/2026-09-05-first-land-surface.md). Final NPC names, room
coordinates and numeric ceilings remain unassigned here.

## Thief guild continuation — September 9

The owner explicitly requires the guild to house the Thief trainer, skill
instruction and Thief spell access, together with the separate dungeon routes.
This is the next authored class venue; its surface entrance and service migration
are not implemented by the four-floor geography slice. Preserve existing teacher
identity and progression state when relocating services. Do not retain a second
mutable copy of guild instruction in the public practice court.

The recovered period Thief guide describes upper-floor magic instruction and
lower-floor weapons instruction, including crossbow, hand combat and dagger,
with secret exits to the first two dungeon depths. The standing class/track
eligibility rules above remain authoritative; precise teacher ceilings, fees and
spell inventory still need their own selected evidence. The guide describes
initial guild entry before learning the secret-door discovery spell, so first
access must not silently require that later spell. Exact discovery versus
physical-door behavior remains a rules fidelity question.

The [gameplay baseline](gameplay-baseline.md) owns sheriff/disguise evidence and
the separate-access direction. The [dungeon execution](plans/2026-09-09-dungeon-one.md)
owns the already encoded basement corridor and reserved surface connection.

## Character creation

Creation profiles are immutable catalog entries selected by the active catalog
profile. Empty selections intentionally offer no creation choices. Required
registry/selection fields replace the previous catalog shape atomically. Each
entry owns its class/nationality baseline sheet, attribute bounds
and point pool, initial resources/skills, equipment, evidence gaps and arrival.
Only the five starting classes are eligible; promotion is not creation.
The September 9 entry direction defers nationality choices in the browser.
It does not remove the existing profile identity or change its mechanical sheet.

The rules validate the whole allocated sheet and loadout against the selected
catalog and world, including bounds and exact point spend. The client submits
only a profile ID, name and six attributes. It cannot choose starting resources,
items, location or a promoted class. The rules instantiate new item identities,
apply normal item binding and prepare a disconnected character without mutating
the current world or consuming its RNG. Allocation changes do not invent a
resource formula: profile resource defaults remain explicit temporary choices
until their dependence on attributes is established.

[Server admission](server-notes.md#character-creation-admission) owns directory
identity, slot allocation, atomic creation/replay and restart reconciliation.
[Browser integration](browser-client.md#private-authoritative-play) owns the
allocation form and transient retry state. Neither path promotes a class or
changes the rules-owned sheet after validation.

## Skill selection and purchase

The player occupies the teacher's square. The right hand selects the requested
track: a weapon selects its weapon family, a lock pick selects thievery, the
appropriate spell book selects class magic, and an empty right hand selects hand
combat. A teacher may refuse a track it cannot teach or a pupil outside its
permitted range. The documented ordinary ceiling for non-Martial Artists is
black belt, with a separate advanced Thief exception to third Dan; this does not
supply every teacher's numeric range.

Training improves subsequent learning and still requires practice. Learned
training survives death even when actual skill is lost. Training purchases also
award experience. Instruction, spell purchase and experience purchase are
distinct transactions; buying experience from a sage does not raise skill.

The documented interfaces include giving gold directly to the teacher and
placing gold on the ground before addressing the teacher. Excess offered gold
is returned to the ground. These interactions are part of the correspondence
target; the current sack-funded typed command alone does not prove both.

The typed training transaction takes the offered amount from the sack, purchases
the absorbable instruction and returns the excess as a ground pile on the
teacher's square. Payment, learning rate, experience and the return commit
atomically. Failure to materialize the return must leave all four unchanged.

Critique reports the current skill level and rank without requiring the focus
item in hand. Weapon families include bow, dagger, flail, halberd, mace, rapier,
shuriken, staff, sword, greatsword and three-sectioned staff. Hand, thievery and
magic are distinct from those weapon families; class magic lanes remain separate.

## Numerical boundary and carried implementation

The practical-guide account describes at most five ranks of prepaid training.
Its worked example at skill nine, rank zero charges 64,000 gold; after reaching
rank one it allows a further 12,800 because four ranks remain prepaid. This is
bounded documentary evidence, not an independently observed server formula.

The carried `engine/training.rs` instead purchases learning-rate units, bounded
by a per-level maximum; `engine/skills.rs` multiplies practice by that rate. The
relationship between this implementation, permanent training and the guide's
prepaid balance needs reconciliation. Do not assert numerical parity because the
existing transaction tests pass, or replace the implementation with a guessed
curve as historical. The owner has authorized temporary retention of this model
for the first expedition under the [provisional-value ruling](gameplay-baseline.md#first-expedition-provisional-values).
Replace it when evidence or an explicit rule selection resolves prepaid balance,
practice consumption and permanent training, with behavioral proof of all three.
Likewise, current combat profiles explicitly label their tuning
`original_provisional`.

Rules own focus/eligibility, training, critique and promotion. Validated content
owns offered tracks and numeric facts. The server persists state and supplies
assessed action options through the protocol. The client displays those facts
and submits the corresponding intent; it does not calculate training gains,
eligibility, balances or progression.

## Evidence bindings and proof obligations

Raw documents remain private. These neutral bindings identify the inspected
documents without requiring an archive to build or test the game.

| Binding | Inspected material and scope | SHA-256 |
| --- | --- | --- |
| `historical-help-111/class-training` | Official release 1.11 help, class topics lines 4–53 and training/critique lines 1829–1990; documentary behavior, not hidden formulas. | `de745be933d1a73f5e8d213e31b5f18fefff3387279b333482fe84b5849a2e1f` |
| `historical-guide-v1/prepaid-training` | Period guide V1 embedded training essay, complete text and worked example; documentary report. | `86df4fd2a908b7cf3a0919533bdd08c995bb24aa17308d4155063fb0ee8b7d80` |

The source descriptions above are independently paraphrased mechanical facts.
The private claim register resolves the neutral documents to exact originals
and preserves release conflicts, including contrary advice about left-hand
training. This contract records the explicit right-hand instructions without
declaring that contradiction resolved.

Observable proof must cover cross-class physical teaching; refusal of the wrong
magic occupation; right-hand focus changes; critique without a held item; trainer
range refusal; atomic payment and excess-gold handling; no immediate skill gain
from instruction; permanent training after skill loss; and promotion refusal for
an ineligible starter. The first expedition additionally requires those services
to work through the actual client and retain state across reconnect.


## Critique feedback projection

The recipient receives a typed critique cue containing service and track
identities, optional track display/title, attained level and optional rank.
The rules project it only for the actor named by the critique event. A missing
rank stays absent. The protocol transports those bounded fields directly and
the private client displays the response without computing skill or rank.
Feedback projection and transaction-receipt conversion are separate children of
the observer owner; adding critique does not enlarge its frame-construction
implementation. The [browser contract](browser-client.md#first-expedition-presentation-study)
owns the current integration and protocol cutover.
