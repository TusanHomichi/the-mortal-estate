---
last_updated: 2026-09-03
revision: 4
status: Owner-authorized standing contract. Revision 3 records the browser-first ruling — the web client's baseline, its feel-surface role, and the retained Godot shell's standing — under the contract's full-re-proof rule; revision 4 records the desktop client as the web client in a native shell.
public_safe: true
summary: The client's standing architecture contract — engine baseline, the three state domains, control and wire consumption, command reconciliation and the epoch cursor, the renderer seam, the accessibility floor, the desktop targets, and the five layers of client proof.
routes:
  - web/**
  - client/**
  - crates/tme-protocol/**
  - tests/fixtures/wire/**
---

# Client architecture

This document owns the client's **standing architecture**: the boundaries that
hold whatever the client looks like.

It does not own the visual target — that is
[presentation direction](presentation-direction.md) — and it does not own the
decisions already implemented and recorded, which are
[client notes](client-notes.md). The split is deliberate: this project is
changing what the world looks like while keeping the architecture, and one
document cannot own both facts through that change.

## Authority

`crates/tme-rules` is the only gameplay authority. `crates/tme-protocol` is the
wire-schema authority. `crates/tme-server` owns authentication, sessions,
characters, sequencing, persistence, admission, and the exhaustive rules-to-wire
conversion.

**The client owns:** input, presentation, audiovisual pacing, ergonomics and
accessibility, application and connection lifecycle, strict wire consumption,
discardable local state, and its own verification.

**The client must not:** import a rules type, inspect content to infer an action,
reproduce legality, advance gameplay time, consume a trace as a live protocol, or
maintain a gameplay, balance, group, offer, quest, resource, or world ledger.

Presentation may explain an authoritative event. It may not patch game truth,
infer hidden legality, keep a competing ledger, or create a second gameplay clock
([boundary map](boundary-map.md#21-the-authoritative-pulse-d5)).

**The beat is the sharpest case of that rule, and the client presents it.**
Ruling D5 permits presentation to remain fluid between authoritative beats and
forbids two things absolutely: a second gameplay clock, and inferring readiness
from elapsed seconds. So the client draws the pulse, under three structural
constraints:

1. **Readiness is the frame's `can_act`.** Nothing else may produce it, and
   elapsed time may never turn a waiting observer into a ready one.
2. **The wait is the frame's own arithmetic** — the whole-round distance between
   its `logical_time` and its `ready_at`, on the digits.
3. **The one local derivation is how far the current beat has got, and it is
   measured rather than declared.** The wire carries no cadence, so the client
   times the interval between the arrival of one round and the next and
   interpolates inside it. It never extrapolates past what it measured, and it
   draws no fill at all until an interval has been observed.

One owner holds all three: `client/presentation/pulse_clock.gd`. Every surface
that presents the beat — the meter, the world view, the feedback director — is
handed that one account, so they cannot describe different beats. The
implementation and its refusals are
[client notes](client-notes.md#the-pulse-made-visible).

## The web client

**Owner ruling, 2026-09-02: browser first.** The client that carries feel,
and the first client the project develops, runs in a browser tab. `web/` is
that client. Every rule in this document binds it exactly as it binds the
Godot shell — authority, the three state domains, strict wire consumption,
the epoch cursor, the renderer seam's four rules, the accessibility floor —
because none of those rules was ever about an engine.

Its baseline:

- TypeScript on Vite, rendering with Three.js on WebGL2, on Node 22 for the
  toolchain. Dependencies are pinned by the committed lockfile and restored
  with `npm ci`; `web/node_modules/` and `web/dist/` are ignored roots
  ([working-root policy](working-root-policy.md#the-roots)).
- **No other runtime dependency without a decision** that owns it, with the
  same licensing, maintenance, and proof obligations the Godot baseline
  carried. A rendering-backend change (WebGPU) is a contract change, not a
  drift.
- The wire codec's intended route is **no mirror at all**: `crates/tme-protocol`
  compiled to WebAssembly and called from TypeScript, so the schema authority
  is consumed rather than re-implemented. Until that lands, the web client
  carries no wire consumption, and any interim TypeScript codec is a verified
  mirror proven against `tests/fixtures/wire/` exactly as the GDScript one is.
- Presentation is judged at the `1280 × 800` minimum play surface in the tab.
  Candidate art reaches the client only through a digest-bound packet named
  out of band (`TME_FEEL_ASSETS` at the dev server), never a tracked path —
  the same rule [presentation direction](presentation-direction.md#the-in-engine-feel-scene)
  states for the Godot feel scene.
- Verification is a lane of its own: the `web` scope in
  `tools/run_verification.py`, gated on the `node` capability, running
  install, typecheck, unit tests, and build; `UNAVAILABLE` without Node.

**The Godot shell is retained and cold.** It remains the reference
implementation of the codec, reconciliation, and HUD contracts, and its proof
keeps running; it is no longer the feel surface, and no presentation
investment goes into it unless a decision gives it a desktop role. Retiring
it is its own decision.

**The desktop client is the web client (owner ruling, 2026-09-03).** The
browser client is the one client; the desktop build is that client in a
native shell — a windowed wrapper giving a Steam build, the Steam Deck, a
gamepad, and an installer to one codebase. The shell is **Tauri** (owner
ruling, 2026-09-03): it keeps the desktop in the Rust ecosystem the server
already lives in, and the owner has shipped with it before. Tauri renders
through each platform's system webview rather than a bundled browser, so the
desktop slice's proof obligation is stated now: the feel client must be
proven on the webview of every desktop target — WebKitGTK on Linux and the
Steam Deck, WebView2 on Windows, WKWebView on macOS — at the ruled play
surface, before that target is claimed. The retained Godot shell stays cold pending a reason the
browser cannot supply, and no other engine enters the stack: Unity was
considered on 2026-09-03 and has no role.

## Engine baseline (the retained Godot shell)

- Godot `4.7.2.stable.official.ed1daf0bf`, project feature `4.7`.
- The `gl_compatibility` renderer.
- Typed GDScript authored here, with `treat_warnings_as_errors` on.
- Godot's built-in HTTP, WebSocket, JSON, TLS, resource, scene, Control, and
  InputMap APIs.

**No third-party addon, C#, GDExtension, embedded native code, or other runtime
dependency.** The tree carries no `addons/` directory and none is expected. A
later decision may widen that boundary only for a measured need, with licensing,
maintenance ownership, and proof on every target platform.

An engine or renderer upgrade is a **direct contract change**: it reruns the
affected import, suite, render, export, and native proof. It does not create a
compatibility branch.

## Three state domains

The client keeps three conceptually separate domains, and each has a file:

| Domain | Holds | Owner |
| --- | --- | --- |
| **Serialised control state** | session secrets, bootstrap, selected character, lifecycle, the active control epoch, the per-epoch sequence cursor, and at most one immutable pending command | `client/adapter/control_state.gd` |
| **Latest authoritative state** | the latest complete observer frame plus accepted welcome/update metadata, with a frame generation counter | `client/adapter/authoritative_state.gd` |
| **Discardable presentation state** | focus, hover, selection, drafts, layout, scrollback, text scale, and other local ergonomics | `client/adapter/presentation_state.gd` |

**Each newer accepted welcome or update replaces the authoritative domain
atomically. Events never patch it.**

Presentation may lag authoritative state for bounded animation — bounded by the
beat, which is where a step's animation both starts and ends. It may not supply
authority, legality, identity, balances, membership, command ordering, or
reconnect state. Authentication secrets never enter presentation state or ordinary
persisted settings — the credential model is
[client notes](client-notes.md#the-credential-model-owner-ruling-d7), and it
persists nothing.

## Control API consumption

**One serialised adapter owns the whole connection lifecycle.** Scenes and UI do
not call HTTP, WebSocket, JSON, cookie, or TLS primitives directly.
`client/adapter/control_facade.gd` holds every route;
`client/adapter/native_transport.gd` holds the transport;
`client/adapter/connection_state_machine.gd` holds the states it may be in.

- Release configuration supplies one HTTPS/WSS endpoint and its exact canonical
  `Origin`. The client performs normal TLS hostname and certificate verification
  and sends that `Origin` on HTTP and WebSocket requests — **without** claiming
  that an `Origin` authenticates native software.
- The session cookie is retained manually and sent on control requests only,
  never on WebSocket traffic. WebSocket admission uses a one-use socket ticket.
- The cookie, CSRF tokens, and tickets are **memory-only**. Passwords never enter
  logs, error text, screenshots, crash text, state summaries, or settings;
  `client/adapter/secret_redactor.gd` enforces it and
  `client/tests/test_secret_redaction.gd` proves it.
- **Control work is serialised** because a successful session bootstrap rotates
  the CSRF token. Endpoint-specific placement is preserved rather than
  normalised: most routes carry the token in strict JSON, and player-kill
  forgiveness carries it in a header with its stable request id in the body.
- **State-changing requests are not generically auto-retried.** After an ambiguous
  outcome the adapter re-runs bootstrap, recovers current control state and a
  fresh token, issues a fresh ticket where one is needed, and then uses only that
  endpoint's own idempotency contract.

The implemented endpoint and schema authorities are
`crates/tme-server/src/control_api.rs` and `crates/tme-protocol/src/lib.rs`. This
document does not duplicate their inventory.

## Strict protocol consumption

The client accepts exactly the current protocol version and the WebSocket
subprotocol `tme.v1`. It obtains a one-use ticket, opens WSS with the configured
`Origin` and that exact subprotocol, sends `client_hello`, and accepts
`server_welcome` only after exact version validation.

**The GDScript codec is a verified mirror of the Rust schema, never a second
schema authority.** `client/adapter/wire_codec.gd` and
`client/adapter/strict_json.gd` implement it, in two stages:

1. Before any dictionary access, raw input is rejected for byte or nesting-depth
   overflow, invalid UTF-8, duplicate keys, and malformed JSON.
2. Typed decoding rejects unknown fields and variants, wrong types, missing
   required fields, invalid required-nullable distinctions, out-of-range or
   non-finite values, non-canonical UUIDs, non-canonical decimal strings, and
   invalid finite enums.

**Identifiers, counters, and wide signed quantities stay canonical strings and
never pass through a float.** The shared conformance corpus covers the boundary
values where a double silently loses precision — 2^53−1, 2^53, 2^53+1, and
`i64::MAX` — plus negative and out-of-range spellings at the applicable type
boundary. Both sides run the same corpus, `tests/fixtures/wire/`, which the client
**reads rather than copies**; the reasoning and the path mechanics are in
[client notes](client-notes.md#the-wire-fixture-corpus-is-read-not-copied).

**No fallback parser, ignored-field mode, alias, or previous-version path exists**
before the external boundary activates. Retired envelope shapes are carried in the
corpus as explicit **reject** cases, so a refusal that used to be free stays
proven.

## Results, updates, and the epoch cursor

The client keeps **at most one immutable gameplay command** awaiting a terminal
result (`client/adapter/pending_command.gd`). A result settles that command by its
id; it does not carry or mutate authoritative state. Only a welcome or an update
replaces the authoritative frame. Ordered observed events drive bounded feedback
and never become a second state store.

The reconciliation rules, in the order they matter:

- The client must not assume the update corresponding to a result is the next
  envelope. It accepts non-contiguous server sequences, ignores an exact duplicate
  update, rejects a lower sequence, rejects an equal sequence with conflicting
  state, and replaces the complete frame atomically on every accepted welcome or
  update.
- **Every accepted welcome creates the active control epoch** and initialises that
  epoch's next client sequence to 1.
- The pending command retains its exact encoded bytes and its correlation,
  sequence, revision, actor, epoch, intent facts, and whether sequence consumption
  has already been applied — until terminal reconciliation.
- Accepted and rules-rejected outcomes **consume** the pending envelope's epoch
  cursor. Wrong actor or epoch, future revision, out-of-order, and projection
  failures **do not**.
- Consumption applies **at most once**, to the cursor of the epoch the pending
  envelope belonged to, and never merely because a replay flag is set. It may
  advance the active cursor only when that pending epoch is still the active one.
  Reconciling an old-epoch receipt settles the old command and its feedback and
  **never** advances the new epoch's cursor.
- A corrected command after a non-consuming rejection uses a new command id with
  the same sequence. A timeout or disconnect may replay only the original
  immutable envelope, under the same id.
- **Reconnect discards cached gameplay authority** and requires a fresh welcome
  with a full frame before ordinary input resumes.

Social messages are transient. Their dispositions are not durable receipts, local
scrollback is presentation state, and display is not proof of delivery.

Proof: `client/tests/test_epoch_cursor.gd`, `client/tests/test_state_reducer.gd`,
and `client/tests/test_path_preview_state.gd`.

## The renderer seam

The world shell owns intent, reconciliation, and every HUD surface. Whatever draws
the world sits behind **one seam** and owns exactly two things: turning an
authoritative frame into something a player can see, and turning a pointer
position into a semantic target.

Four architectural rules bind any implementation:

1. **Everything the seam asks for is expressed in world squares and frame rows** —
   never in meshes, cameras, or pixels. A view is given a rectangle; it is never
   told about the HUD.
2. **A view never sends a command and never mutates authoritative state.** It
   emits pointer facts; the shell decides what they mean.
3. **Targets come from one neutral owner**, so every consumer sees one identity
   space regardless of what drew them.
4. **A view may spread motion across a beat and may not manufacture one.** The
   shell hands it the beat's progress; a view uses it to animate or ignores it.
   It never times a beat itself, never anticipates the next one, and never lets
   a bounded animation change which square a thing is on.

The seam's member contract, the current implementation, and the capture obligation
a view inherits are owned by
[client notes](client-notes.md#the-world-view-seam). A replacement renderer
substitutes behind the same seam and inherits the same obligations unchanged.

## Input and the accessibility floor

- Every non-text action uses a namespaced semantic input action. Bindings are
  remappable and **persisted separately from authentication state**
  (`client/input/binding_store.gd`).
- The shell is keyboard-operable with visible deterministic focus and focus
  restoration after a modal closes. Each pointer operation gains a keyboard route
  when its owning surface implements that operation.
- **Essential state must not rely on colour alone.** A cue that only exists as a
  hue is not a cue.
- The shell remains operable at `1280x720` with **enlarged text**, and readable at
  higher resolutions. `client/tests/test_input_bindings.gd` asserts the exact
  minimum viewport at 200 percent text scale with no clipped essential control.
- Text scale and audio preferences are a versioned document that **fails back to
  defaults** rather than accepting a shape it does not recognise.
- A destructive or irreversible confirmation names the action and its target, does
  not focus the confirming control by default, and becomes invalid when
  authoritative state changes underneath it.

Exact typography, the contrast palette, motion and flashing controls, cue timing,
and complete gamepad operation are later work. The floor above is not.

## Desktop targets and proof

The project targets Linux x86_64, Windows x86_64, and macOS universal. Platform
behaviour stays behind a narrow adapter: gameplay, protocol, state, presentation,
and input do not fork by operating system. Client packaging is independent of the
server release cadence.

**A named target is not a support claim.** A platform is proven when an exported
artefact reaches the real TLS/WSS smoke boundary on that native system; until then
it is target-only, and partial proof stays labelled partial. Installers, stores,
auto-update, crash reporting, public distribution, and web or mobile exports are
later product decisions.

Export presets exclude `tests/*`, which is why test-only code may use a
source-tree path honestly.

## The five layers of client proof

| Layer | What it proves | Where |
| --- | --- | --- |
| 1 | strict codec, lifecycle, and state-reducer behaviour, headless | `client/tests/test_strict_json.gd`, `test_wire_codec.gd`, `test_connection_state_machine.gd`, `test_state_reducer.gd`, `test_epoch_cursor.gd` |
| 2 | one shared positive/negative wire corpus consumed by **both** sides | `tests/fixtures/wire/`, asserted from Rust in `crates/tme-protocol/src/client_fixture_tests.rs` and from GDScript in `client/tests/test_wire_codec.gd` |
| 3 | scene, input, focus, accessibility, and minimum-resolution behaviour | `client/tests/test_input_bindings.gd`, `test_full_hud.gd`, `test_domain_panel.gd`, `test_interaction_director.gd` |
| 4 | a real server through the real contracts, from an empty database | `tools/run_client_live_proof.py` driving the shipped `ClientRoot.tscn` |
| 5 | controlled render, capture, and export evidence | `client/tests/capture_fixture_frame.gd`, `client/tests/live_capture.gd`, `client/tests/pulse_capture.gd`, `client/tests/validate_export_presets.gd` |

The table is the retained Godot shell's proof; the web client's standing
proof is the `web` lane plus the real-tab walk proof and captures described
in [client notes](client-notes.md). The packaged Tauri client adds a layer of
its own when the desktop slice lands: the packaged app launched on each
target and the same walk proof driven through its webview. Until that layer
is green for a target, the target is not claimed, whatever the other layers
say.

Layer 4 is the one that matters most and the one most easily faked. It exists
because **constructing a component in a test is not the same as exercising it
through the wiring the product uses** — the lesson recorded in
[agent workflow](agent-workflow.md#prove-the-real-path-not-a-reconstruction-of-it).

Structural, state, focus, and layout contracts are automated. **Visual judgment is
not**: it uses controlled same-environment captures or reviewed references, never
cross-machine byte equality.

Only individually reviewed resources with suitable licensing, provenance, and
ownership may cross into `client/`. Private reference material is not a client
seed, a schema authority, a content source, or cross-platform proof
([public boundary policy](public-boundary-policy.md)).

## Revisit boundaries

Each of these needs a new explicit decision and its own proof:

- replacing the engine, or widening the dependency baseline — for the web
  client, adding any runtime dependency or changing the rendering backend;
- changing the renderer;
- giving the retained Godot shell a desktop role, or retiring it;
- persisting any credential (today: nothing is persisted);
- admitting outside players, or activating external compatibility;
- adding distribution infrastructure.

The protocol-first, server-authoritative boundary is what keeps each of those
bounded rather than structural.
