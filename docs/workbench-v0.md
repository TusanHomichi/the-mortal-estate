---
last_updated: 2026-09-05
revision: 5
status: Logical and recorded-capture selection retained; Godot fresh-capture runtime retired September 5. Browser capture integration remains open.
public_safe: true
summary: Workbench pointing, projection selection, packets, capture addressing, staleness, retention, and proof.
routes:
  - tools/workbench/**
  - tools/workbench_demo.py
  - tools/workbench_prune.py
  - tests/test_workbench_*.py
  - tests/test_capture_*.py
  - tests/fixtures/workbench/**
  - tests/fixtures/capture/**
---

# Workbench V0 — the Selection Bridge

The owner points at a place. An agent receives an exact, stable,
machine-resolvable address instead of a prose description it has to guess at.

There are two surfaces to point at and they answer with the same address:

- the **logical view**, the authoring compiler's own projection of the compiled
  land;
- the **capture view**, a real photograph of a real client frame, with the
  identity of every pixel recorded beside it.

That is all of V0, and it is deliberately all of it. **The editing half is
[Workbench V1](workbench-v1.md)**, which owns the session directory, the
staged-operation log, Apply, and the image operations; everything below is about
pointing, and where the two touch, V1 is the owner. Its governing authority is
the successor design spec *The Mortal Estate — Workbench Selection Bridge*,
which lives in the planning tree rather than in this repository; §11.2's
out-of-scope list binds here in full, and §11.3's acceptance criteria are the
proof set below. This document records what was built, not what was decided.

Fresh capture is not currently implemented. The owner retired its Godot runtime;
existing capture evidence remains loadable and selectable.

## What it does not do

Not a single one of these is a "not yet implemented" apology; each is a bound
limit of this slice.

- **No staged operations, no Apply.** Nothing here can change tracked content.
  V1 adds both, and still writes no tracked content — see
  [Workbench V1](workbench-v1.md).
- **No candidate validation consumer.** `validate_candidate` exists in
  the compiler and V0 does not call it, because V0 produces no candidate. V1 is
  its first consumer.
- **No image operations, no adapters, no candidate promotion.** A capture is a
  context image and is bound as one; `commit_mask` stays null, because nothing
  may be replaced.
- **No dressing or asset views.**
- **No multi-user and no remote access.** One owner, one machine, loopback.
- **No second renderer.** The logical view draws the compiler's projection and
  says so in its own banner. The capture view draws a picture the client took
  and nothing else — no overlay of the compiler's opinion on a photograph. The
  Workbench never approximates a gameplay frame; it shows a real one or it shows
  the projection, and it labels which.
- **Recorded captures are diagnostic evidence.** Their retired producer drew
  flat-colour cells. They are not visual masters; [browser client](browser-client.md)
  owns the current scene.

## Running it

```bash
cargo run -p tme-authoring            # produce the logical projection (once, ahead of time)
python3 tools/workbench/serve.py      # serve both views on 127.0.0.1:8730
```

The default projection is the synthetic authoring fixture. To inspect the
identity-proof land, select its tracked projection explicitly:

```bash
python3 tools/workbench/serve.py \
  --projection content/lands/identity-proof/generated/workbench_projection.json
```

`--port 0` picks a free port; `--root` points at another checkout; `--session`
names the session directory instead of taking a fresh one.

Selecting never builds anything. The projection is a tracked artifact of the
authoring compiler, and the Workbench reads it; if it is missing the server
refuses to start and names the command above.

The capture view reads existing session captures. There is no renderer launcher,
fresh-capture button, or `POST /api/capture` command. A browser producer must
satisfy the capture contract below before fresh capture can return.

To see the whole loop at once, including both views and the agent read path:

```bash
python3 tools/workbench_demo.py
```

## The agent read path

An agent needs no browser and no server.

```bash
python3 tools/workbench/resolve.py .workbench/sessions/<id>/selections/sel-0001.json
python3 tools/workbench/resolve.py <packet> --json        # machine-readable
```

`resolve.py` refuses before it answers. It recomputes every bound digest, checks
the mask against the cells it claims to encode, **re-derives the gesture** from
the geometry the packet recorded, and re-derives the packet's identities from the
current projection to confirm the recorded ones. Exit `0` resolved, `2` refused,
`3` unreadable.

A capture packet gets three further checks, because it binds three further files.
The picture, the sidecar, and the identity raster must still describe each other;
the sidecar's frame generation, camera and viewport must be the ones the packet
recorded; and the gesture is replayed **through the raster**, so the cells and the
observed identities are recomputed from the capture rather than trusted from the
packet.

The session directory is plain files. Listing it, reading a packet with any JSON
tool, and resolving it are all ordinary file operations — that is what makes
agent parity mechanical rather than aspirational.

## The selection packet

One packet is one act of pointing. Written to
`<session>/selections/<selection_id>.json`.

**One record type, two views.** A logical selection and a capture selection are
the same record with different values, never different shapes. The fields a
capture fills are present and empty on a logical packet, so a consumer written
against one reads the other without a branch.

| Field | What it carries |
| --- | --- |
| `schema_version`, `kind`, `workbench_version` | `1`, `workbench_selection_packet`, `v0` |
| `selection_id` | stable within the session (`sel-0001`) |
| `created_at` | wall clock, for human reading only — nothing resolves by time |
| `author` | `owner`; the field the operation log shares |
| `source.repository_revision` | the checked-out commit, **advisory** (see below) |
| `source.digest_binding` | `fail_closed` |
| `source.digests` | five bound files for a logical selection; **eight** for a capture selection, which adds the picture, the sidecar, and the identity raster |
| `view` | `logical` or `capture` |
| `scene` | `land_id`, `realm_id`, `member`, and `frame_generation` (null for logical, the photographed generation for a capture) |
| `camera` | null for logical; for a capture, the lattice pitch, the square origin in viewport pixels, the square bounds, and the viewport size |
| `screen_region` | the gesture, the **space** its geometry is in, the **geometry** exactly as it was made, the reported canvas rectangle, the derived cell bounds, and a mask reference for lasso and paint |
| `cells` | the exact covered cells, row-major, in the master's coordinate frame |
| `world` | the authored cell lattice, the tile pitch, and the pixel bounds that follow from it |
| `semantic` | the ranked identity set (below) — **identical for both views over the same region** |
| `candidates` | the same ranked set when `ambiguous`, empty otherwise — a view of `semantic`, never a second list |
| `ambiguous` | whether a consumer must ask rather than pick |
| `observed` | what the photographed frame showed under the gesture; empty for a logical selection |
| `context_image` | the capture's picture, by path and digest; null for logical |
| `commit_mask` | null; V0 has no image operations |
| `comment` | the owner's words, verbatim, never parsed for facts |

### Why the revision is advisory and the digests are not

A working tree is routinely dirty while the owner works. Refusing every
selection taken on uncommitted work would be theatre, so the revision orients a
reader and the **digests decide**. Any bound digest that moves is a refusal
naming the file, from every consumer, with no nearest-match fallback and no
re-resolution. A selection that silently followed a moved world would convert a
precise instruction into a confident wrong one.

### Why the geometry is recorded

The packet records the gesture as it was made — the clicked point, the dragged
rectangle, the lasso polygon, the painted path — in the space the view uses. A
consumer re-derives the cells from it rather than trusting the `cells` field.
Without that, a hand-edited cell list would resolve happily to a place the owner
never pointed at.

### `semantic` names the land; `observed` names the moment

The semantic set names the authored world: structures, transitions, landmarks,
routes, terrain, and layers, resolved from the compiler's projection over the
covered cells. That is why a capture selection and a logical selection over the
same region carry an identical semantic set — the whole of acceptance criterion
2.

What the frame happened to be showing at the instant of the photograph — an actor
mid-step, a corpse, a dropped coin, and the squares themselves — is real, is worth
recording, and is not an authored identity. It goes in `observed`. Folding it into
`semantic` would break that equality and invite a consumer to address a transient
as though it were a place.

### Semantic identities

Each identity names what occupies part of the selection, with two coverage
ratios: `selection_coverage` (how much of what you pointed at is this thing) and
`identity_coverage` (how much of the thing you pointed at).

| Kind | Identity | Detail |
| --- | --- | --- |
| `structure` | `structure:<member>:<id>` | footprint, access cell, façade door, purpose, scope, and which of them the selection touched |
| `transition` | `transition:<member>:<id>` | target member, paired transition, direction, marker and access cells |
| `landmark` | `landmark:<member>:<id>` | role and position |
| `route` | `route:<member>` | route membership; the authored routes layer carries no per-route id, so this names membership rather than inventing one |
| `cell_terrain` | `terrain:<member>:<class>` | the terrain class, its authored layer, and how many covered cells are passable |
| `layer` | `layer:<member>:<layer>` | which authored layer the address belongs to, so a truth edit and a dressing edit over the same cells stay distinguishable |

Ranking is by share of the selection, then by share of the identity, then by
kind, then by name — total and stable. **Ranking is a presentation convenience.
Ambiguity is data.**

### The ambiguity rule

A selection is ambiguous when any of these holds:

1. more than one *occupant* identity (structure, transition, landmark, route)
   covers it — two buildings, or a landmark standing on a route;
2. exactly one occupant covers it but does not account for the whole selection,
   so the rest of what was pointed at is unexplained;
3. no occupant covers it and it spans more than one base terrain class.

It errs toward asking. Over-flagging costs one question; under-flagging spends
an agent's confidence on the wrong address.

### Masks

Lasso and paint selections write a mask beside the packet: a P1 portable bitmap
over the selection's bounding box, with the origin in a comment line. Plain text,
no library to read or write, and referenced by path and digest — so a mask edited
after the fact stops matching the address it belongs to, and `resolve.py`
refuses. A capture lasso writes the same mask over the same cell bounds: the mask
is an address, not a picture.

## The capture path

Recorded captures support selection and correspondence checks. Browser screenshots
and wire observer recordings have separate proof routes in
[verification](verification.md#on-demand-proofs); neither is an authoritative
browser image/identity/sidecar capture yet.

### What a capture is

Three files, written together by the client's own presenter, meaningless apart:

| File | What it holds |
| --- | --- |
| `capture.png` | what the presenter drew, straight off the viewport |
| `capture.identity.pgm` | the **identity raster**: one 16-bit index per pixel naming which entry of the target list owns that pixel, or zero for none |
| `capture.sidecar.json` | frame generation, camera identity, viewport size, the digests of the other two files, and the full target list — identity, kind, coordinate, presentation layer, screen anchor, and screen hit shape |

The raster is a binary Netpbm greyscale (`P5`, maxval 65535, most significant
byte first) because both sides then need no library at all: a producer writes pixel identities, and the Workbench slices bytes with
`struct`. The picture is a PNG, and the Workbench reads exactly one thing from it
— the width and height in its header — which is ten lines of `struct` rather than
an imaging dependency.

### How a gesture becomes an address

A gesture is pixels. A pixel is a target index, read from the raster. A target
stands on a square. A square is a cell of the compiled member. Nothing along that
chain is estimated, and the last step is asserted cell by cell in
`tests/test_capture_correspondence.py`: a frame row at world position (x, y) **is**
master cell (x, y) of that member, with no offset, in both members, with a mutant
proving the assertion has teeth.

Coverage is per pixel and is not weighted: one pixel of a square is that square.
The logical view behaves the same way at cell granularity, which is what keeps the
two views' answers comparable.

Clicking an occupant's marker resolves to the square it stands on and records the
occupant in `observed`. That is the case a nearest-anchor scheme gets wrong.

### Recorded correspondence evidence

The two retained captures show the same authoritative fixture land under two
framings. Their images differ while selected identities agree, proved by
`tests/test_capture_addressing.py::TheTwoCaptureRoutesResolveIdentically` and
`tests/test_capture_correspondence.py`. The input frame is
`tests/fixtures/capture/fixture_land_frame.json`. This proves those recorded
artifacts, not the currency of a renderer that no longer exists.

## The two plan-level rulings

The genesis plan gave this slice's implementation two of the spec's open
decisions to rule on with evidence rather than on principle.

### Decision 3 — identity raster **and** target list, not anchor-only

**Ruled: ship both.** The spec recommended anchor-only for V0, with the raster as
the first V0.1 item if capture-side lasso proved imprecise in real use. That
recommendation was written against the native-3D presenter, where a raster meant
a second render pass over a scene graph. It does not survive contact with a 2D
presenter.

The reasoning, in the order it decided the question:

1. **The raster costs almost nothing to write.** Every drawn thing is already an
   exact rectangle, so the raster is those rectangles filled in draw order, in
   engine code (`Image.fill_rect`). It adds no measurable time to a 3.9-second
   capture and 8 KB to a repository once compressed (1.5 MB raw, and it is a
   16-bit buffer of long constant runs).
2. **It costs almost nothing to read.** A click through the raster resolves in
   **0.33 ms** — the same as a logical click. A box over a seven-by-five block of
   squares takes **5.3 ms**; a box over the entire 1024×768 frame takes **22 ms**.
   Rows are read as slices, not pixel by pixel.
3. **It removes a whole class of wrong answer.** Anchor-only means lasso and paint
   over a capture are nearest-anchor guesses. With the raster they are exact, and
   an occupant marker overlapping the square beneath it has one owner per pixel
   rather than a tie-break rule someone has to remember.
4. **It makes the sidecar checkable.** Because the raster is exactly the target
   rectangles in draw order, a consumer can rebuild it and compare — which is what
   `tests/test_capture_sidecar.py::TheRasterAndTheTargetListAreOneFact` does. A
   target list alone can only be believed.

The target list ships too, and not merely as metadata: it is what the raster
indexes, it carries each target's coordinate and presentation layer, and it is
what `observed` is drawn from. The sidecar carries `frame_generation`, the camera
identity, and the viewport size either way.

**The measured cost that would have changed this** is the one that did not appear:
a raster large enough to dominate the capture or the repository. At one 16-bit
sample per pixel it is neither.

### Decision 7 — drive the cheap route; keep the expensive one as the reference

The separation remains a capture-design obligation: replaying a recorded frame
is distinct from observing the live server, and neither belongs on the ordinary
selection path. Both old Godot producers are retired. The
[dated capture evidence](plans/2026-08-20-workbench-capture-evidence.md) records
their measured costs and correspondence; it supplies no current run command.

## The cost of every loop, measured

Selection reads existing projection/capture files and starts no process.
`tools/workbench_demo.py` exercises that path using the recorded fixture and
reports its timings. Historical renderer measurements are in the linked receipt
above. Fresh browser capture will need new measurements.

## The session directory

**Owned by [Workbench V1](workbench-v1.md#the-session-directory)**, which
describes it whole. What V0 puts in it is the manifest, the packets, the masks,
the captures, and two record kinds in `operations.jsonl`:

```json
{"schema_version":1,"kind":"selection_recorded","record_id":"op-0001","recorded_at":"…","author":"owner","selection_id":"sel-0001","packet":"selections/sel-0001.json","operation":null}
{"schema_version":1,"kind":"owner_comment","record_id":"op-0002","recorded_at":"…","author":"owner","selection_id":"sel-0001","comment":"…","operation":null}
```

`operation` was the seam V0 left for V1, and V1 fills it with a class, a verb,
and parameters. On these two kinds it stays `null`, and a reader may rely on
that.

### Retention

**Sessions are disposable working state.** They are never tracked, never runtime
input, never an authority. A session left open for a week commits to nothing.

- Keep the session you are working in; drop the rest.
- `rm -rf .workbench/sessions` is always a safe command in this repository.
- Nothing tracked references a session, so no cleanup can break a build, a test,
  or a proof.
- **§13 open decision 2 is closed.** The retention ruling — sessions older than
  fourteen days, and any session beyond the most recent ten, are removed — is
  owned by [the working-root policy](working-root-policy.md#the-retention-ruling)
  and implemented by `python3 tools/workbench_prune.py`. The numbers are
  restated in every session manifest and asserted by
  `tests/test_workbench_session.py::Retention`.

Captures live inside a session and share its retention: they are working state,
not evidence. The two tracked captures under `tests/fixtures/capture/` are the
exception and are proof material with their own provenance record.

## D6 compliance

The ignored-lane ruling, requirement by requirement, **for the Workbench**.
[The working-root policy](working-root-policy.md) owns D6 for the whole ignored
root and is the authority on any conflict; this table is how V0 meets it.

| D6 requirement | How V0 meets it |
| --- | --- |
| Tracked builds, tests, and boundary proof never depend on the ignored root | No test reads `.workbench/`. Proofs use the tracked authoring fixture, the tracked synthetic fixture, and the tracked capture fixtures; the tests that write a real session write it, assert, and delete it |
| Clean clones carry tracked synthetic fixtures for automated proof | `tests/fixtures/workbench/` — a complete synthetic land, session, packets, masks, and log, generated by `regenerate.py` and byte-checked. `tests/fixtures/capture/` — two real captures of one real frame, so the capture path proves out with no client binary and no display |
| Missing private fixtures produce an honest unavailable result, never a false pass | Missing/malformed projections and captures raise explicit errors. No test claims a fresh render; the retired capture command is refused rather than redirected to a fixture. |
| Session files carry source hashes and cannot become runtime input | Every packet and manifest carries its bound digests. Nothing in the runtime, the compiler, or the content path reads a session; the manifest states `runtime_input: false` |
| Retention and cleanup policy are explicit | Above; ruled in [the working-root policy](working-root-policy.md#the-retention-ruling), implemented by `tools/workbench_prune.py`, and restated in every session manifest |
| Accepted outputs enter tracked content only through the promotion path | V0 produces no output that could be accepted. It cannot write tracked content at all |

## Position in the architecture

The Workbench sits in the Tools boundary. Its four hard limits, and what holds
each one:

| Limit | What holds it |
| --- | --- |
| Never a runtime input | Sessions live under an ignored root; nothing outside `tools/workbench` and its tests reads them |
| Never a second gameplay authority | Passability, terrain classes, footprints, doors and reachability all come from the compiler's emitted projection. The package parses no authored document. A capture's identities come from the client's own presenter, not from anything recomputed here |
| Never a second content ledger | The projection is derived, `--check`-verified output of the compiler, not a ledger. Tracked geography keeps its single owner |
| Never a second renderer | The logical view draws cells and says so. The capture view draws a photograph the client took and nothing else. Neither approximates the game |

Its endpoints bind loopback, refuse a non-loopback client or `Host` header, and
are not a service boundary. No external-compatibility, versioning, or migration
policy activates because this exists.

## Criteria and their proof

Spec §11.3, all ten, with the phase that owns each.

| Criterion | Phase | Proof |
| --- | --- | --- |
| **1. Pointing resolves exactly** | 4W | `tests/test_workbench_pointing.py` — 14 cases over the accepted fixture, one per gesture plus features; expected covered cells and full identity lists written out, read off the authored document by hand |
| **2. Capture selections resolve to the same address space** | 6W | `tests/test_capture_addressing.py::ACaptureSelectionResolvesLikeALogicalOne` — click, occupant click, box, lasso and paint over a real capture, each compared against the equivalent logical selection; and `::TheTwoCaptureRoutesResolveIdentically`, which does it again between two captures of one frame at different scales. `tests/test_capture_correspondence.py` asserts the underlying claim: runtime cell (x, y) **is** master cell (x, y), cell by cell, with an offset mutant |
| **3. The identity sidecar is real and matching** | 6W | `tests/test_capture_sidecar.py` — the sidecar's viewport equals the picture's own PNG header; the raster is the same resolution; the raster rebuilt from the target rectangles equals the raster byte for byte; every target's anchor pixel names that target; a marker owns its pixels over the square beneath. Browser producer/pointer correspondence remains an integration obligation |
| **4. Staleness fails closed, per digest** | 4W + 6W | `tests/test_workbench_staleness.py` — each of the five bound files mutated independently, killing the packet in `verify`, in `resolve.py`, and over HTTP; plus a deleted source, an edited mask, and a hand-edited packet. `tests/test_capture_addressing.py::ACapturePacketFailsClosedPerDigest` — all **eight** mutated independently, plus an edited cell list, an edited observed list, an edited gesture, an edited frame generation, and an edited camera |
| **5. Ambiguity is data** | 4W | `tests/test_workbench_ambiguity.py` — all three clauses of the rule exercised separately, both directions, plus rank stability and a four-occupant selection |
| **6. Agent parity holds** | 4W + 6W | `tests/test_workbench_parity.py` — the real HTTP server's answer compared against `resolve.py` run as a separate process, for every gesture; plus reading the tracked fixture cold. `tests/test_capture_addressing.py::AgentParityHoldsForCapturePackets` — the same, for capture packets, for every gesture |
| **7. The loop is fast where it must be** | 4W + 6W | `tests/test_workbench_loop.py::test_selection_to_written_packet_stays_in_milliseconds` guarantees the selection path. The capture cost is **measured and recorded above**, from real runs, and reported in the interface — never asserted |
| **8. Nothing canonical moves** | 4W + 6W | `tests/test_workbench_session.py::NothingCanonicalMoves` — a full four-gesture logical session in this repository with `git status` compared before and after. `tests/test_capture_addressing.py::NothingCanonicalMovesWhenSelectingOverACapture` — the same for a capture session |
| **9. No full verification in the loop** | 4W + 6W | `tests/test_workbench_loop.py` parses selection modules and drives real routes with a process-spawning tripwire. Only `bridge.py` may invoke the authoring compiler; the retired fresh-capture route is refused. |
| **10. The comment survives** | 4W | `tests/test_workbench_session.py::TheCommentSurvives` — a comment with newlines, quotes, trailing whitespace, non-ASCII and fake structured data, checked byte for byte through the packet, the log, and the agent consumer, with a proof that none of it reached a typed field |

The compiler-side emitter has its own proofs in
`crates/tme-authoring/tests/workbench_projection.rs`: determinism across runs,
the tracked bytes equal to a fresh build, every named source digest equal to the
file on disk, every cell present exactly once in row-major order, passability
equal to the compiler's own count, and layer attribution asserted per cell.

## Honest gaps

1. **World coordinates are cell coordinates.** The logical frame is the authored
   cell lattice; `world.pixel_bounds` follows from the compiler's own tile pitch.
   A capture binds a real screen frame in its `camera`, but the authored world
   still has no metric frame of its own, and V0 does not invent one.
2. **Routes have no per-route identity.** The authored routes layer is a tile
   layer, so `route:<member>` names membership. If routes ever become typed
   objects, the identity becomes `route:<member>:<id>` and nothing else changes.
3. **`elevation` / `layer_band` is null.** This land separates surface and
   interior as members, not as bands within one member, so there is nothing
   honest to put there.
4. **A capture shows what was in view.** The frame the client is sent is bounded
   by the observation radius, so a capture addresses the squares that were
   visible, not the whole member. Pointing outside them is a refusal, not a
   guess. The logical view addresses the whole land and is the right surface for
   a region nobody is standing in.
5. **Fresh authoritative browser capture is missing.** Tracked as
   [issue #43](https://github.com/TusanHomichi/the-mortal-estate/issues/43). Owner: the browser
   renderer integration. Required proof: live and replayed frames produce matching
   image/identity/sidecar artifacts; pointer and raster identities agree; replay
   corresponds to the observed authoritative frame. Existing fixtures cannot
   discharge this proof.
6. **`canvas_rect` is reported, not verified,** for logical selections. The screen
   rectangle is the client's own account of where the gesture happened; the cells
   are the address. For a capture selection the gesture geometry **is** verified,
   because the cells are derived from it.
7. **Colour carries no authority in either view.** The logical view spreads hues
   across the projection's terrain classes. Recorded diagnostic captures also
   use arbitrary colours. Identity comes from the sidecar and target list,
   never from a pixel's colour.
