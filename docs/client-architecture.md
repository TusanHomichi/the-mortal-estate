---
last_updated: 2026-09-05
revision: 12
status: Three.js browser runtime with a Rust WebAssembly codec and read-only authoritative capture; production control UI remains open.
public_safe: true
summary: Browser authority, shared Rust WebAssembly decoding, state domains, renderer identity, production integration targets, and proof limits.
routes:
  - web/**
  - crates/tme-protocol/**
  - tests/fixtures/wire/**
---

# Client architecture

This document owns the client's **standing architecture**: the boundaries that
hold whatever the client looks like.

[Browser client](browser-client.md) owns current implementation and operation;
[presentation direction](presentation-direction.md) owns the visual target.
The owner retired Godot on September 5. The browser is the sole client runtime;
a read-only authoritative browser observer now supplies diagnostic Workbench
capture. Production control/UI integration remains unimplemented.
Contracts below describe obligations, not claims of completed browser features.

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
([boundary map](boundary-map.md#21-authoritative-individual-deadlines-d5)).

**Cooldown presentation follows each action's authoritative deadline.**
Ruling D5 requires independent deadlines. Readiness comes only from the frame's
`can_act`; local elapsed time cannot unlock an action. Remaining time is the
canonical decimal millisecond difference between `logical_time` and `ready_at`.
The client may interpolate progress within that reported interval, clamped at
completion, while it waits for the authoritative ready frame.

## The web client

**Owner ruling, 2026-09-02: browser first.** The client that carries feel,
and the first client the project develops, runs in a browser tab. `web/` is
that client. Every rule in this document binds it — authority, the three state domains, strict wire consumption,
the epoch cursor, the renderer seam's four rules, the accessibility floor —
because none of those rules was ever about an engine.

Its baseline:

- TypeScript on Vite, rendering with Three.js on WebGL2, on Node 22 for the
  toolchain. Dependencies are pinned by the committed lockfile and restored
  with `npm ci`; `web/node_modules/` and `web/dist/` are ignored roots
  ([working-root policy](working-root-policy.md#the-roots)).
- **No other runtime dependency without a decision** that owns it, with the
  same licensing, maintenance, and proof obligations. A rendering-backend change (WebGPU) is a contract change, not a
  drift.
- The wire codec uses **no mirror at all**: `crates/tme-protocol`
  compiled to WebAssembly and called from TypeScript, so the schema authority
  is consumed rather than re-implemented. This is implemented for the diagnostic
  observer through `web/src/authoritative/codec.ts`. Both native and WebAssembly
  decoders execute the complete shared corpus in `tests/fixtures/wire/`. The
  page sees decoded objects only after strict Rust validation.
- The accepted layout and display-scaling target is owned by
  [presentation direction](presentation-direction.md#proportional-scaling);
  [browser client](browser-client.md) separates that target
  from the current browser proof surface. Candidate art reaches the client
  only through a digest-bound packet named
  out of band (`TME_FEEL_ASSETS` at the dev server), never a tracked path —
  the [candidate-asset rule](presentation-direction.md#candidate-assets).
- Verification is a lane of its own: the `web` scope in
  `tools/run_verification.py`, gated on the `node` capability, running
  install, typecheck, unit tests, and build; `UNAVAILABLE` without Node.

**Every real-tab proof runs in two engines (owner ruling, 2026-09-03).** The
owner reviews the preview in Firefox; the proofs and captures ran in headless
Chromium. A picture judged in one browser is judged in both: the walk proof
and the capture tool run in Chromium and in Firefox through Playwright, each
engine with its own server, tab, and captures. Firefox requires a display for
WebGL; [browser client](browser-client.md#renderer-capability) owns
the hardware/software launcher and its capabilities. A run that cannot supply
an engine or its requested renderer is incomplete, never a pass on the other. The desktop webviews join
that list as targets are claimed. Narrowing a run to one engine is a look,
not a proof.

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
surface, before that target is claimed. No second engine or client runtime is retained.

## Three state domains

The production client must keep three separate state domains:

| Domain | Holds |
| --- | --- |
| **Serialised control state** | session secrets, bootstrap, selected character, lifecycle, the active control epoch, the per-epoch sequence cursor, and at most one immutable pending command |
| **Latest authoritative state** | the latest complete observer frame plus accepted welcome/update metadata, with a frame generation counter |
| **Discardable presentation state** | focus, hover, selection, drafts, layout, scrollback, text scale, and other local ergonomics |

**Each newer accepted welcome or update replaces the authoritative domain
atomically. Events never patch it.**

Presentation may lag authoritative state for bounded animation — bounded by the
action interval, which bounds a step's animation. It may not supply
authority, legality, identity, balances, membership, command ordering, or
reconnect state. Authentication secrets never enter presentation state or ordinary
persisted settings — the credential model is
[credentials](#credentials-owner-ruling-d7).

## Control API consumption

**One serialised adapter owns the whole connection lifecycle.** Scenes and UI do
not call HTTP, WebSocket, JSON, cookie, or TLS primitives directly.
The browser integration must provide that adapter before UI surfaces send commands.

- Release configuration supplies one HTTPS/WSS endpoint and its exact canonical
  `Origin`. The client performs normal TLS hostname and certificate verification
  and sends that `Origin` on HTTP and WebSocket requests — **without** claiming
  that an `Origin` authenticates native software.
- The session cookie is sent on control requests only,
  never on WebSocket traffic. WebSocket admission uses a one-use socket ticket.
- The cookie, CSRF tokens, and tickets are **memory-only**. Passwords never enter
  logs, error text, screenshots, crash text, state summaries, or settings.
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

The browser codec consumes the Rust schema authority through WebAssembly,
with both targets proven against the shared corpus:

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
boundary. `tests/fixtures/wire/` remains one shared positive/negative corpus.
The native and browser WebAssembly decoder tests consume that corpus directly.

**No fallback parser, ignored-field mode, alias, or previous-version path exists**
before the external boundary activates. Retired envelope shapes are carried in the
corpus as explicit **reject** cases, so a refusal that used to be free stays
proven.

## Results, updates, and the epoch cursor

The client keeps **at most one immutable gameplay command** awaiting a terminal
result. A result settles that command by its
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

Browser integration must prove these cases through the actual connection adapter.

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
4. **A view may animate within the supplied action interval.** The
   shell hands it cooldown progress; a view uses it to animate or ignores it.
   It never owns readiness, never anticipates acceptance, and never lets
   a bounded animation change which square a thing is on.

The diagnostic authoritative renderer supplies matching color, identity raster,
and sidecar output for [Workbench addressing](workbench-v0.md#what-a-capture-is).
Its neutral target owner uses observer rows; the same meshes supply the image,
GPU identity pass, and raycast pointer targets. It draws no candidate artwork
and implements no gameplay controls. The candidate-packet feel scene remains
a separate local experiment awaiting production authoritative integration.

## Input and the accessibility floor

- Every non-text action uses a namespaced semantic input action. Bindings are
  remappable and **persisted separately from authentication state**
  independently of session secrets.
- The shell is keyboard-operable with visible deterministic focus and focus
  restoration after a modal closes. Each pointer operation gains a keyboard route
  when its owning surface implements that operation.
- **Essential state must not rely on colour alone.** A cue that only exists as a
  hue is not a cue.
- The shell remains operable at `1280x720` with **enlarged text**, and readable at
  higher resolutions. Prove the minimum viewport at 200 percent text scale
  without clipping essential controls when the production UI is implemented.
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

Production packaging must exclude proof fixtures, test code, and private packets.

## Proof surfaces

| Surface | What it proves | Limit |
| --- | --- | --- |
| `web` verification lane | Typecheck, browser unit tests, build | Synthetic inputs; no live server |
| Two-engine walk proof | Real Three.js movement, cursor, facing, portals and rendering | Candidate packet; no authoritative wire |
| Native and WebAssembly protocol corpus | Current and refused wire formats through the same Rust codec | No production control UI |
| `tools/run_server_live_proof.py` | Real TLS sign-in, admission, land, individual cooldowns, reconnect, logout | Python wire observer; no rendered browser claim |
| PostgreSQL gated suite | Durable server, session, restart, and recovery invariants | Requires a scratch database administrator |
| `tools/run_browser_capture_proof.py` | Native WSS, live/replay browser images, raster/pointer identity, source binding and Workbench HTTP selection in both engines | Diagnostic renderer; scratch TLS certificate errors permitted only in ephemeral proof profiles |

Production browser integration needs end-to-end tests through shipped UI and
transport wiring. A server wire proof cannot stand in for that. Tauri targets
add native webview and packaging proof before support is claimed. Visual
acceptance remains the owner's; it is not inferred from passing tests.

## Credentials (owner ruling D7)

Persist no credentials in project settings or local files. Passwords, CSRF
values, and admission tickets remain transient; session handling must respect
the server's cookie and token contracts. Any durable credential store requires
a separate owner decision and a platform secret facility. Keep preferences
and bindings separate from authentication. There is no saved-login import path.

## Revisit boundaries

Each of these needs a new explicit decision and its own proof:

- replacing the engine, or widening the dependency baseline — for the web
  client, adding any runtime dependency or changing the rendering backend;
- changing the renderer;
- persisting any credential (today: nothing is persisted);
- admitting outside players, or activating external compatibility;
- adding distribution infrastructure.

The protocol-first, server-authoritative boundary is what keeps each of those
bounded rather than structural.
