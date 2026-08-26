---
last_updated: 2026-08-27
revision: 4
status: Authored at genesis plan Phase 2 under owner ruling D1; extended at slice S1 and accepted for the clean public-source cut while its remaining gate continues to bind the G11 release.
public_safe: true
summary: What content/test-corpus/ is and is not — explicitly non-canonical neutral test data, barred from defining the settlement, the dead world, public lore, or release content.
routes:
  - content/test-corpus/**
---

# Test Corpus Provenance

`content/test-corpus/` is **non-canonical test data**. It is the conformance
corpus the rules, protocol, and simulation crates are proven against. It is not
world canon, not lore, and not release content.

This document is the record every fixture in that corpus points at. Each
scenario's `research_boundary.review_refs` resolves here and to its own
`*.provenance.md` sibling, so the provenance chain of the corpus terminates
inside this repository rather than in a tree that no longer exists.

## What this corpus is

209 files: 52 scenario roots, their 52 `*.provenance.md` records, 52 simulation
seeds, 52 world templates, and one catalog. Four of five predecessor crates
depended on it, three of them at compile time. It arrived here with the rules
spine because the two cannot be separated: `cargo test` fails at the compiler,
not at an assertion, if the content is missing.

## What the shared catalog also carries

`catalogs/prototype_catalog_v6.json` is the only definition registry this
project carries, and since slice S1 an authored land outside this corpus
resolves against it. Two entries in it therefore belong to the identity proof's
land rather than to a scenario here —
`actor-definition/identity_proof/threshold_keeper` and
`service/identity_proof/threshold_keeper`, both selected by
`profile/first_land_structure`, both placeholder role labels under owner ruling
D2.

That does not make this corpus canonical and does not make that land test data.
It records an arrangement forced by there being exactly one registry: a second
copy would be a second source of truth for the same fact class. The arrangement
ends when a production content registry lands, which is a recorded gap in
[the authoring compiler](authoring-compiler.md).

## Where it came from

The corpus was authored clean-original by this project's predecessor. It was
never copied research output, never converted map or extracted data, and never a
source payload — the per-fixture records preserved alongside each scenario state
that in their own words, and their `clean_content` / `research_boundary`
structure is carried here unchanged.

## The ruling this corpus migrated under

The owner ruled on 2026-08-19 (G0 decision **D1**, executed at the **G4** gate)
that the corpus migrates as an explicitly non-canonical, neutral test corpus.
The ruling closed two alternatives it was offered: a period with no Rust test
coverage was not accepted, and re-authoring all 209 files before migration
merely for aesthetic purity was not accepted either.

The ruling's requirements, and how each is discharged:

| Requirement | How it is met |
| --- | --- |
| Classified as test data, not world canon | This document, and the README in the corpus root. |
| Predecessor settlement and realm branding removed | See **Debranding** below. |
| **Every** `review_refs` entry resolves | Every entry names this document and, where one exists, the fixture's own provenance record. Proven by `tools/check_review_refs.py`. |
| Behavioral coverage and deterministic fixtures preserved | Every golden was regenerated from the migrated corpus by the harness and is byte-identical to the predecessor's, with identifiers normalized. |
| May not define the first settlement, dead world, public lore, or release content | The identifiers are transparently synthetic (`testland`, `test_realm`-shaped ids). Nothing here names a place this project intends to build. |

The clean public-source audit evaluated the carried corpus rather than
waiving that bullet. Every path and byte passes the provisioned real private
denylist at both enforcement points, the provenance chain closes inside this
tree, and the corpus remains explicitly non-canonical. It is therefore accepted
as source-neutral and public-safe for repository visibility.

That result does **not** promote the corpus into release content. The G11
external release must still exclude it or separately authorize an exact
allowlisted role for it. See **Carried lineage signals** below for context that
remains inappropriate as product authority even though it is safe to read.

## Debranding

Every transformation was found by search, never by eye, and applied by one
re-runnable script. The classes:

| Class | Change | Sites |
| --- | --- | --- |
| Realm and land identifiers | the predecessor settlement name → `testland` | 21,243 |
| Content service ids | `*_healer`, `*_temple` re-prefixed `tme_` | 34 |
| Placeholder marker | the enforced on-disk data marker → `TME-PLACEHOLDER` | 13 |
| Creature id | one cliff-dwelling creature id → `raptor`; the predecessor's name for it is a source proper noun carried on the private denylist | 11 |
| Prose | predecessor product name → `project` / `project-authored` | 160 |
| Lineage prose | two records named the source game in prose; rewritten to state the same fact without naming the lineage | 2 |
| Corpus path | `content/prototypes` → `content/test-corpus` | 86 |
| Provenance doc references | predecessor `docs/` paths → this document | 24 |

The 52 `*.provenance.md` records keep their structure, their evidence
statements, and their boundary claims. What changed in them is branding, the
doc paths they cited, and the two lineage sentences named above. The one record
that instructed a reader to run the predecessor's authoring compiler was
rewritten: that compiler and its authored master are retired under decision
**D3** and are not carried here, so the instruction described a build this
repository cannot perform.

## Carried lineage signals

Stated plainly, because the pre-public-snapshot gate will have to deal with
them and a silent carry-forward is how these things get shipped:

- Roughly ten fixture records carry the phrase "commercial 1.11" in **negative**
  assertions ("these are clean-fixture values, **not** commercial 1.11
  constants"). No banned term is involved and the honest disclaimer is worth
  keeping until the fixtures themselves are replaced, but the phrase is a
  version reference to an external client.
- Two assertion messages in `crates/tme-rules/tests/cases/xp_progression.rs`
  described a growth total using a source-community faction name. Adjudicated
  at the Phase 4 port: renamed to "baseline-progression" and the name added to
  the private denylist, so any reintroduction fails the boundary check.

## Boundary enforcement

Two independent checks enforce the source-term boundary, and neither carries a
real term in this tree:

- `tools/check_boundary_terms.py` scans **every carried file**, path and
  contents, against the private denylist in the git-ignored `.boundary/` root.
- `crates/tme-rules/src/content/validation/boundary/` scans content at **load
  time**, including content that never enters this tree, against the data file
  named by `TME_BANNED_TERMS_FILE`.

Both fail closed when their term file is missing, unreadable, or empty after
parsing. Both prove their rejection mechanism against the invented nonsense
terms in `tests/fixtures/synthetic-terms.txt`. That fixture is what lets a clean
clone build and test green with the private root absent — the tree carries the
mechanism, never the terms.
