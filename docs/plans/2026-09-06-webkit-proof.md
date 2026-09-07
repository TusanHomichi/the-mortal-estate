---
last_updated: 2026-09-06
revision: 1
status: Implementation complete; portable, web and Linux native verification passed; packaged desktop proof remains separate.
public_safe: true
summary: Shared engine roster, WebKit launch and GPU evidence, trust lifecycle, integration proof and desktop limits.
---

# WebKit proof expansion

The owner requested WebKit coverage in preparation for the Tauri desktop client.
[Client architecture](../client-architecture.md#the-web-client) owns the expanded
matrix and separate packaged-webview obligation. [Browser client](../browser-client.md#renderer-capability)
owns launcher and evidence details; [verification](../verification.md#capabilities)
owns availability and verdicts. This plan owns execution and the local receipt.

## Scope

Add WebKit to one executable engine roster and migrate capability probes,
private-play proof, candidate captures and authoritative Workbench production
and verification together. The Workbench must offer every required live/replay
capture atomically. Preserve missing-engine and requested-renderer refusal.
Do not change gameplay, candidate art, the packaged preview or Git state.

Search the live owners for `chromium`, `firefox`, `PROOF_ENGINES`, `both engines`,
`two-engine`, fixed four-capture expectations and installation instructions.
Historical receipts retain their observed two-engine results.

## Findings resolved in the implementation

- The launcher treated every non-Chromium browser as Firefox. WebKit now has its
  own GTK launch path and resizable display shell.
- GTK font DPI silently rescaled the requested viewport on the headless display.
  Disposable 96-DPI settings and a native CSS-viewport check resolve that cause.
- WebKit's JavaScript adapter name is masked. It cannot prove GPU use. A native
  Linux probe checks a rendered WebGL2 pixel and increasing DRM counters in this
  launch's tagged WebKit content/GPU process, excluding the compositor.
- WebKitGTK does not use the NSS trust store. Private-CA proof temporarily adds
  one unique system trust anchor and removes it on stop/failure. Normal TLS
  validation remains active; unavailable privilege/tooling refuses launch.
- Python capture producers and UI proofs carried separate engine/count lists.
  They now consume the shared roster and derive expected batch sizes from it.

## Proof and limits

Run the selected repository lanes, then native launch/refusal and TLS lifecycle
checks, full-roster authoritative capture/Workbench proof, and candidate shader,
fade, camera, walking and interior regressions. Preserve external logs, source
identity and screenshots. Record exact results at closeout.

Linux GTK is the current native WebKit proof environment. A masked adapter with
no native backend evidence is unavailable, including currently unproven software
and non-Linux WebKit configurations. Playwright's WebKit build is engine coverage;
each claimed Tauri target still needs packaged tests on its actual system webview.

## Observed closeout

Portable and web verification completed, including 457 tests in 37 browser test
files, Rust workspace checks, Python proof groups and real private-boundary checks.
Native Linux hardware proof passed all three engines for exact layered-surface
compositing, six authoritative live/replay captures, Workbench pointer selection
and recording, and the installed two-tab private client. The latter retained
normal TLS validation and covered independent action deadlines, cooldown
reconnect, movement, logout revocation, transient authentication and enlarged text.

WebKit passed day/dusk camera and doorstep checks, all 837 compiler-mask verdicts,
sea refusal, fifteen arrival targets, meadow interaction, foreground fading and
restoration, and the courtyard/interior/portal regression. Native screenshots
were inspected. Deliberate software-renderer and incorrect-DPI substitutions
were refused; temporary GTK settings and certificate trust were cleaned up.

The external receipt binds the final source and evidence hashes. The original
candidate manifest matches its preceding sealed receipt; no gameplay, candidate
art, packaged preview or Git lifecycle changed in this slice. The scratch database
used for native capture is local proof infrastructure, not a second live world.
Packaged platform proof remains owned by the desktop slice; the limitations above
are carried in the browser-client owner.
