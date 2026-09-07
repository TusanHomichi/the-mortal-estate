---
last_updated: 2026-09-06
revision: 7
status: Owner-amended for historical gameplay reconstruction and geographic derivation; publication boundaries remain separate.
public_safe: true
summary: Source/product boundaries, the first-land map-template exception, external payloads, provenance, and naming.
routes:
  - content/**
  - docs/**
  - tools/check_boundary_terms.py
  - tools/check_clean_room.py
---

# Public boundary policy

## The premise

This repository is the **clean public-source successor**. It was created from an
explicit allowlisted export of the reviewed project tree. Its parentless public
root inherits no commit, object, branch, issue, pull request, Actions run,
release, deployment, cache, or other collaboration record from either private
repository that preceded the public cut. The private development repository is
an owner-held read-only archive; the frozen private predecessor remains private
and unchanged.

Two boundaries must not be conflated:

1. **Source publication** makes this allowlisted tree and the collaboration
   record created from its public root readable. The public cut occurred only
   after the source and the private development container were audited.
2. **External product publication** distributes a game, client, content snapshot,
   service, public API, store page, or other release surface. It remains closed
   until its own gate authorizes it.

Everything below follows from that separation. The source tree must hold its
boundary every day, including while public; a release must additionally prove
the exact artifact and surface it distributes.

The default requirement, from the charter, is that the public or public-bound
tree follows the rules below. The later owner rulings in
[Historical gameplay reconstruction](#historical-gameplay-reconstruction) and
[First-land map template](#first-land-map-template) supersede those rules in
their stated scopes:

- builds and runs without private reference material, provider credentials, or
  historical prototype roots;
- contains only original project expression, reviewed reusable code, and properly
  licensed third-party material;
- avoids copied or lightly disguised names, text, geometry, coordinates, tables,
  schemas, artwork, and other distinctive payloads from any other work;
- keeps private reference material outside the public dependency graph;
- records provenance and licensing for every promoted third-party or generated
  asset; and
- **fails closed** when a check detects a prohibited dependency or term.

This document owns the policy. [Boundary checks](boundary-checks.md) owns the
machinery that enforces it, and is not duplicated here.

## Historical gameplay reconstruction

**Owner ruling (2026-09-06).** Reconstruct the selected historical game's exact
gameplay on the modern stack with modern graphics and the explicitly authored
living/dead-town changes. Restore or recreate the research workspace for that
purpose. The [gameplay baseline](gameplay-baseline.md) owns fidelity accounting
and deviations; the [surface brief](plans/2026-09-05-first-land-surface.md#living-town-dead-town-and-dungeon)
owns the town and dungeon arrangement.

This supersedes D2's blanket fresh-design requirement for baseline gameplay.
Agents may recover and inspect private research, extract mechanical facts,
compare versions and author specifications and original implementation against
that evidence. The target is observable gameplay; historical code, protocols,
storage and scheduling internals are not reconstruction requirements. Matching behavior, formulas and supported numerical values is
permitted. Deliberate divergence is not a condition of acceptance. Prior original
design decisions remain visible with explicit dispositions; provisional values
and closed research questions are not automatically exact historical facts.

Permitted geographic derivation also covers the first land's historical town as
a dead-world quest space, its four initial dungeon levels and the three deeper
historical levels reserved for later content. The existing geographic provenance
and authoring requirements apply to every layer and connection.

Research stays in an external private workspace. Source files, client binaries,
extracted artwork, archived prose and bulk reference inventories stay there.
Self-contained authored gameplay contracts may carry established mechanical
facts and neutral evidence identities/digests; private records resolve those
identities to exact sources. Public specs state the resulting rule without
requiring private access. Builds, tests and runtime have no archive dependency.

Within this scope, this ruling supersedes the default reference-as-specification
prohibition, human-only conclusion step and payload restriction for mechanical
facts and authorized geographic derivation. Copied code, art, audio, dialogue,
branding and other expressive payloads receive no import permission. Naming,
private spoilers, provenance, licensing and publication gates continue to apply.
Private research may name sources; public naming restrictions must not prevent
that investigation or erase design continuity. This ruling is project direction,
not a claim of rights in third-party material.

## First-land map template

**Owner ruling (2026-09-05).** The owner explicitly changed the boundary and
directed use of the selected historical land map as the first land's template.
This permits direct geographic derivation, including tracing or retaining its
overall shape, terrain arrangement, route topology, relative landmark positions,
and coordinates. Recognizable layout and close correspondence are allowed;
independent invention or deliberate distortion is not a condition of acceptance.
The [surface design brief](plans/2026-09-05-first-land-surface.md) owns subsequent
land-design decisions about what is retained or changed.

This section is the sole owner of the exception. It supersedes the charter's
carried original-geometry restriction, the map-shape homage limit, the general
payload and reference-as-specification prohibitions, and D2's fresh-design
requirement for these geographic facts. An agent may inspect and transcribe the
selected map for that purpose, including with tools. The later [gameplay ruling](#historical-gameplay-reconstruction) extends this
scope; remaining material follows the default policy.

Raw source maps, screenshots, source files and extracted reference inventories
stay in an external quarantine root. Derived first-land layouts, geographic
specifications and authored map data may enter the tree through the ordinary
authoring and promotion process. They must be described as template-derived,
with provenance recording the inspected source identity and digest, the method,
and subsequent changes; they must not claim independent geographic authorship.
Source identification that crosses the naming guardrail stays in the external
provenance record, with a neutral identifier and digest in carried records.
The authored output must build and run without the external reference root.

The ruling authorizes the geographic template; it does not establish a
third-party license or waive required attribution. Names, story text, artwork,
characters, gameplay tables and mechanics receive no import permission from
this geographic exception. Mechanical facts follow the later
[gameplay ruling](#historical-gameplay-reconstruction). Naming, hidden-world spoiler exclusion, asset provenance,
authoring acceptance and external product-publication gates remain in force.
Generated geography within this scope follows the same provenance and promotion
requirements as manually derived geography.

A map-derived specification must identify and inspect its exact source artifact
and version before claiming correspondence. The
[surface brief](plans/2026-09-05-first-land-surface.md#verified-template) records
the selected template's neutral identity and digests. This policy amendment
imports no raw map and accepts no finished land.

## Kinds of material

| Kind | Where it may live | May it reach the tree? |
| --- | --- | --- |
| **External reference material** — files, assets, text, names, coordinate sets, geometry, bulk numeric tables, schemas, vocabularies, screenshots, recordings from someone else's work | a local-only quarantine root outside this checkout and its dependency graph | No raw payload; the [first-land exception](#first-land-map-template) governs its permitted derived outputs |
| **Authorized template-derived geography** | authored first-land documents and map data, with provenance | Only within the [first-land exception](#first-land-map-template) and ordinary authoring/promotion process |
| **Authorized mechanical facts** | self-contained authored gameplay contracts and conformance cases with neutral evidence identities | Within the [gameplay reconstruction ruling](#historical-gameplay-reconstruction); no raw research or historical-code dependency |
| **Private working material** — local prototypes, candidates, captures, session state, credentials, the private denylist | an ignored working root inside the checkout | No, except where a tracked file must exist to define or ignore it |
| **Project expression** — original design, code, content, and documents authored here | the tree | Yes, that is what the tree is |

A quarantine root is **outside this checkout, local-only, and not required to
begin work**. A clean clone has none, and everything tracked must build, test,
and run without one. That is
not a convention; it is the property the clean-room check exists to prove.

## The payload rule

This is the default definition; the [reconstruction ruling](#historical-gameplay-reconstruction)
permits the mechanical facts and geographic derivation it names.

> **Using reference material to inform an originally written rule or value is
> ordinary design work. Copying or reformatting a payload — even under new names,
> even into cleaner JSON, even retyped by hand — is an import.**

Reformatting does not launder. A table transcribed into a different serialisation
is the same table. A coordinate set with the names changed is the same coordinate
set. A schema redrawn with new field names is the same schema.

What crosses is a **conclusion a human wrote**, in this project's own words, into
this project's own contracts. What never crosses is the expression, the data, or
the identity.

## The quarantine flow

```text
external material -> inspection -> a human-authored conclusion -> project content
```

Outside the [authorized reconstruction scope](#historical-gameplay-reconstruction)
and [first-land exception](#first-land-map-template), every arrow is a
decision, and the middle one is a person. Nothing traverses this
flow automatically, and no tool may be built that does — an automated path from
reference material to project content is exactly the thing the boundary forbids,
whatever it is named.

Concretely:

1. Reference material is inspected in the quarantine root, by a person or an agent
   explicitly asked to.
2. The inspection may produce notes, reports, or extracted facts. **That output is
   not project content**, and it stays in the quarantine root.
3. A conclusion is written by a human — as a design decision, in original words,
   with the project's own vocabulary and structure.
4. Implementation is then clean and original **against that written conclusion**,
   not against the material.

A remembered detail is a **design lead until it is decided**. It is never promoted
to a fact because it fits the intended shape well. A lead that turns out to be
wrong costs a design conversation; a lead promoted to a fact costs a mechanic
nobody can trace.

## The marker contract

Any artefact that is not clean project content must **declare itself**. The label
matters more than the format, and the test is simple: can a future contributor
tell what this file is without re-reading this document?

- Prefer keeping the artefact in an ignored root, where its location is the label.
- Where a file-level marker is appropriate, add one in a header comment naming
  what the file is and that it is not project content.
- Where a tracked inventory is more natural than a per-file marker, keep the
  inventory and keep it current.

**Never commit a prototype artefact as ordinary project content.** An unmarked
artefact is indistinguishable from clean content the moment the person who made
it stops paying attention, which is the failure this contract exists to prevent.

The [first-land exception](#first-land-map-template) and
[gameplay reconstruction ruling](#historical-gameplay-reconstruction) own the
authorized derivation lanes. Raw external payloads have no general tracked lane.
Mechanical facts enter through self-contained authored contracts with evidence;
restoring the research archive does not import its old runtime registrations.

## The evidence ladder

When two sources disagree about what is true for this project, the higher rung
wins:

1. **An explicit owner ruling.**
2. **The charter**, and later owner-approved product direction.
3. **This repository's own contracts, tests, fixtures, and goldens.** Implemented,
   proven behaviour outranks any prose about it, including prose in this
   repository.
4. **Original design work and observed play.** A play session is evidence; an
   argument about how a system should feel is not.
5. **Reviewed, properly licensed third-party material**, with its provenance and
   licence recorded.
6. **External reference material outside the [authorized reconstruction scope](#historical-gameplay-reconstruction).** It may inform a human-authored conclusion. It
   is **never** the basis of a specification, never a payload source, and **never
   appears in a document as the authority for a value.**

Two rules about the ladder itself:

- **Preserve conflicts; never silently merge them.** When two sources disagree and
  both matter, record both, record the conflict, and record which one this project
  selected. A merged average of two claims is a third claim nobody made.
- **Nothing is promoted automatically.** Later, newer, or more detailed does not
  mean higher. A rung is a rung.

Subject to the [gameplay reconstruction ruling](#historical-gameplay-reconstruction)
and [first-land exception](#first-land-map-template), historical ruling **D2**
binds every rung above: all exact mechanics, names, timings,
penalties, and routes are reopened for fresh design, and **no document in this
project may retain a private research route as authority.** Not as a citation, not
as a footnote, not as a parenthetical.

## Recognisable inspiration, and where it stops

This project may be obviously in a lineage, and may keep the qualities that make a
game of this kind good: deliberate pacing, dangerous terrain, corpse recovery,
social dependence, long progression, old-school tension. Those are a genre, not a
payload.

Common vocabulary is common. Weapon categories, creature and service archetypes,
and generic mechanics such as reach, ranged attacks, readiness, and
terrain-constrained movement are not derived payloads by themselves.

Outside the [authorized reconstruction scope](#historical-gameplay-reconstruction)
and [first-land exception](#first-land-map-template), a homage is allowed
when it is independently authored and clearly part of this
project's own expression. It should read as a nod, not as a near-copy of an asset,
a location, a character, a table, a passage, a map shape, a packet schema, or a
brand claim.

**The boundary is expression, data, and identity.** The age, availability, or
shutdown status of any other work changes none of it.

## Generated content

AI-assisted generation is a standing production tool here, not a banned asset
class — and it is **not a route around the quarantine**.

- Assets generated from this project's own accounts and inputs may become tracked
  content when they win on merit. Each carries per-asset provenance: the tool or
  service, the meaningful inputs, the date, and the licence basis for commercial
  use.
- Third-party or community AI-library assets are someone else's generations under
  their own terms. Each needs a licence check **before even reference use**, with
  the terms recorded beside the provenance.
- Except for geography under the [first-land exception](#first-land-map-template),
  generation prompted with, seeded by, or visibly reproducing another work's
  expression stays on the far side of the quarantine, exactly as the material
  itself would. A model is a copying mechanism when you point it at something.
- A model's knowledge of other games is **not a clean content source**. Persistent
  generated text needs constrained generation or human review before it becomes
  durable world text — the same rule the AI boundary states in
  [boundary-map.md](boundary-map.md#part-3-ai-is-never-game-authority).

A slice may adopt a stricter boundary than this where it serves the work.

## The naming guardrail

Public-facing surfaces must not name another game, publisher, descendant server,
or their marks. That includes README text, repository descriptions, packaging,
store and website copy, code identifiers, in-game strings, file names, and commit
messages.

**New public-facing artefacts use neutral project naming from their first draft.**
Renaming later is how a name survives in a filename, a fixture, or a history
nobody re-reads.

Product-name coupling stays shallow in the other direction too: the title belongs
in public metadata, executables, UI, packaging, website, and release surfaces.
Deep architecture uses functional subsystem names. No slice introduces a new
product-name-derived identifier into a schema, wire string, environment variable,
or type name.

## The standing spoiler exclusion

**Owner: the project owner. This rule cannot be satisfied by an agent's judgment
that a document reads harmlessly.**

Spoiler-bearing hidden-world truth — the answers behind mysteries, the identities
behind unresolved questions, the face-down cards of the design — is **excluded from
source publication and every external publication surface** by standing rule, and
stays private absent an explicit later owner decision to publish a curated
version.

This is ruling D2's fourth clause and it carries forward unchanged. Two
consequences worth stating:

- A document that would be public-safe on every other axis is still excluded if it
  carries a face-down answer. Cleanliness is not the test; disclosure is.
- The exclusion is a property of the **content**, not of the directory. Moving a
  spoiler into a differently named file does not resolve it, and neither does
  paraphrasing it.

Every document in this tree carries a `public_safe` field in its front matter. It
is a claim, and like any claim it is the author's responsibility to be able to
defend.

## The clean public successor and the two publication cuts

The first proposed public cut was an in-place visibility change of the private
development repository. Independent review refused it because one reachable
commit-message token crossed the private-lineage guardrail. Rewriting its refs
would not prove removal from cached GitHub views or pull-request references. On
2026-08-27 the owner therefore authorized the safer disposition:

- merge the accepted source tree and preserve the development repository as a
  private read-only archive;
- export only the reviewed carried tree, never its `.git` directory or
  collaboration state; and
- create this public repository from that export with a parentless root.

That is not a filtered history and does not launder the rejected object. No
private Git object was copied at all. Historical commit, issue, and pull-request
identifiers retained in planning records are opaque private-archive receipts;
they are not objects in this repository, do not imply public access to the
archive, and grant no authority beyond the owner ruling already captured in the
tracked document.

The source-cut audit covered more than the checked-out tree:

1. **Audit every Git ref and reachable object**, including commit and tag
   messages, file contents, and path names.
2. **Audit repository collaboration state**: issues, pull requests, review and
   issue comments, Actions logs and artifacts, releases, deployments, caches,
   variables, environments, hooks, keys, and repository metadata.
3. **Provision the real private denylist out of band** and run both enforcement
   points against it. Synthetic CI terms prove the mechanism, not the carried
   material.
4. **Review disclosure and provenance**, including every `public_safe` claim,
   generated or third-party asset, license, opaque provenance link, and
   spoiler-bearing fact.
5. **Verify a clean clone** with no private root and record any capability that
   is unavailable rather than calling it a pass.

The owner records the result and alone authorizes a cut. A finding involving a
secret, private term, spoiler, copied expression, unlicensed material, unresolved
name conflict, or unrepeatable protection rule blocks it. That rule is why the
in-place cut was refused and this clean export exists.

An external product publication is a second cut. It uses an explicit allowlisted
artifact or surface: allowlisted, not "everything except." It excludes every
private root, credential, working artifact, internal record not deliberately
published, and source-only fixture not authorized for release. Proof runs against
the staged artifact and its fresh consumer before any package, build, service,
store presence, or release is announced.

Required third-party licence notices and asset provenance remain mandatory. No
cleanup may delete or falsify attribution to manufacture a clean result.

Neither this policy nor source publication authorizes an external product
publication on its own.

## What enforces this

Five fail-closed checks, run by `python3 tools/run_checks.py`:

| Check | Defends the rule |
| --- | --- |
| banned-terms | the naming guardrail, in file contents **and** file names |
| review-refs | provenance chains that resolve inside this tree |
| hostnames | real external infrastructure named in carried files |
| clean-room | dependence on a private root, and private roots becoming committable |
| markdown-links | broken or escaping local document references |

Their exact semantics, allowlists, exit codes, and the mutant kill that qualifies
each one to block are owned by [boundary checks](boundary-checks.md).

Three properties of that machinery matter to this policy and are worth stating
where the policy lives:

- **Exit 3 means FAIL CLOSED, and it is never a skip and never a pass.** A check
  whose configuration is missing, unreadable, or empty stops and says so. A check
  that goes quiet when its input disappears is worse than no check, because it
  reports green while defending nothing.
- **The mechanism is public; the terms are private.** The denylist lives in an
  ignored root and never enters history, and a missing denylist fails the check
  closed. A fresh clone therefore fails that check until the owner provisions the
  file out of band. **That is the intended first experience, not a setup bug.**
- **There is a second enforcement point.** The checks scan what the repository
  carries; they cannot see content that never enters the tree. The rules crate
  therefore carries the same convention at content-load time, with the same format,
  matching rule, and fail-closed semantics.

## Known limits

Stated plainly, because a boundary whose gaps are unrecorded is a boundary people
trust further than it deserves:

- **The checks scan text, not meaning.** They catch a name, a host, a path, an
  orphaned provenance reference. They cannot catch a paraphrase, a redrawn map, or
  a table retyped with new labels. Those are caught by review, and only by review.
- **A clean-reading document can still be provenance-tainted by process.** Where
  the design started matters even when the words are original — the reason the
  predecessor's first-step authoring rule was deleted outright rather than
  adapted ([agent workflow](agent-workflow.md#what-this-replaced-and-why)).
- **`public_safe` is a claim by the document's author.** Nothing verifies it
  mechanically. The clean break's audit is where it is actually tested.
