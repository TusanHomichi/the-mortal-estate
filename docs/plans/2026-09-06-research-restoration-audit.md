---
last_updated: 2026-09-06
revision: 2
status: Archive restoration and scoped verification complete; gameplay fidelity remains feature-specific and incomplete.
public_safe: true
summary: Private archive integrity, independent static checks, recovered decisions, evidence limits and follow-up ownership.
---

# Research restoration and audit

The owner supplied the historical research archive and requested renewed audit,
including verification of earlier model-authored conclusions. The
[gameplay baseline](../gameplay-baseline.md) owns the selected target: reproduce
the player-experienced game with modern graphics and technology. Original code,
wire formats, database structures and scheduling machinery are not requirements.
The [public boundary](../public-boundary-policy.md#historical-gameplay-reconstruction)
owns private research and authored derivation; the
[surface brief](2026-09-05-first-land-surface.md#living-town-dead-town-and-dungeon)
owns the explicitly changed world.

## Restore and evidence identity

The externally held input is **research-archive-20260906**, SHA-256
`f72b109c2cc2ee1cebc921973e13d3fa7f2b419cd6dca2aac6f7caf607870d20`.
It contains 3,294 files, 3,494,255,666 compressed bytes and 4,209,168,278
extracted bytes. Entry-path checks, all ZIP CRC checks and a second SHA-256 read
of every extracted file passed. The original archive and extracted originals
remain unchanged outside the repository.

The private workspace now contains a current reading index, full restored-file
manifest, source dispositions, all 104 prior question dispositions, class/trainer
recovery and the detailed audit with reproducible check scripts. Earlier research
instructions are historical evidence; the maintained owners here govern work.

## Observed findings

- The historical retained-file manifest has 1,830 matching entries, 21 changed
  entries and no missing files. Forty-four files are outside that manifest. Its
  old inventory cannot serve as a present integrity pass; the new archive-wide
  receipt preserves the discrepancy rather than rewriting historical hashes.
- All 61 JSON, 212 JSONL, 328 TSV and 124 CSV files parsed. All 21 top-level
  source routes and all migration targets exist. The wider 71-route review found
  a referenced historical specification, now recovered from its pinned Git blob,
  and a stale taxonomy-version path, now recorded with its versioned successor.
- All 33 source/projection inputs match the paired final ledger's hashes. All
  55 claim collections reproduce their counts and state totals, covering 77,272
  rows. Those labels include coverage and design dispositions, so they cannot
  be reported as a percentage of gameplay reproduced exactly.
- A newly written binary reader, using no archived decoder imports, reproduced
  all 74,820 map cells and 299,280 layer slots from six primary databases. Seven
  raw catalog record counts match the retained exports. This checks map values
  and catalog completeness, not every catalog field's semantic interpretation.
- A separate executable-data reader reproduced 30 character-creation attribute
  rows and five allocation pools. Inspection of the corresponding disassembly
  checked the initialization and cap path. Retained official help supports five
  starting occupations plus a promotion, and the actual teaching relationships.
- The historical seven-level dungeon text map is present, including the three
  lower levels reserved for later content. Historical map correspondence and
  playable connections still require authored proof.
- Of the 104 prior question IDs, 98 were closed through successor design
  ownership, five as bounded matches and one by reframing. The five match IDs
  include one duplicated cross-system fact and version-specific qualifications.
  These dispositions preserve useful work; they do not establish whole-game
  historical equivalence.

The audit found strong static evidence and substantial recoverable design
context. Major remaining uncertainties concern player-visible combat, progression,
training, effects, ecology and service behavior. Historical server internals
need not be recovered when equivalent behavior can be demonstrated directly.
Mixed source versions and descendant comparisons must remain explicit.

## Scope and next proof

Archive-wide integrity, structured-format checks, registered source routes,
decision classification and the stated independent static checks are complete.
The audit did not replay every observation, inspect every prose claim, or prove
all runtime behaviors. A recovered file is not an accepted mechanic.

| Finding | Owner | Required next proof |
| --- | --- | --- |
| Generic trainer layout omitted recovered teaching relationships | Gameplay and town authoring | Class/skill/service matrix with explicit gates and town sites; real client transactions. |
| Prior design closure mistaken for historical certainty | Gameplay research | Per-feature source/version, observable cases, actual implementation and explicit differences. |
| Map storage addresses mistaken for elevation | Geography authoring | Four-level connectivity, town-interior exceptions and separate dead-world layer. |
| Deferred lower dungeon content | Geography authoring | Historical lower-map reconstruction and connection correspondence before authoring acceptance. |
| Stale historical manifests and version route | Research tooling | Preserve old receipts; use the new full manifest and explicit source-version dispositions. |

The current index routes these findings without changing the original archive.
The standing policy, workflow, boundary map and entry routes now reflect the
owner's reconstruction direction. No gameplay code, candidate artwork, Git
lifecycle or deployed preview changed in this audit.

Documentation and boundary verification use the repository runner for this slice.
The private closeout records the exact command result and artifact hashes.

## Private research reference

The owner requested a shared editable reference optimized for agent retrieval,
with a browser reader and small source-linked images. The private capability
`tme-research` now provides ranked text search, stable claim lookup, exact source
passages, source-hash freshness checks and revision-preserving article edits.
Its entry point is `~/.local/share/tme-reference/README.md` on the development
machine. No build, test or runtime depends on that optional private capability.

The claim register separates owner decisions, official documentation, player
observations, bounded static verification, former design choices and open gaps.
Articles summarize those records and reference their IDs. Accepted product facts
remain in their existing repository owners; the reader exposes those documents
through explicit read-only routes. The original archive remains read-only through
the tool. Browser saves preserve history and reject conflicting revisions.

A fresh bounded web search recovered seventeen files with dated receipts and
found both new leads and source contradictions. A recovered calculator has mixed
provenance; some high-level growth values are explicitly extrapolated; historical
rankings contain inconsistent level assignments. None establishes a complete
historical formula. Full sources, exact URLs and per-claim limits remain private.

The reference is a loopback-only local service, with an SSH tunnel for owner
access. It is not a published site or an off-machine backup. This slice adds
research tooling outside the checkout and does not alter the deployed preview.
