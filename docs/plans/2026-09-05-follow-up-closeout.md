---
last_updated: 2026-09-05
revision: 1
status: Historical closeout; issues 42 and 43 implemented and the full local baseline passed.
public_safe: true
summary: Completed follow-up implementation, diagnosed findings, observed failures and repairs, and full local verification evidence.
---

# September 5 follow-ups

Scope: resolve [#42](https://github.com/TusanHomichi/the-mortal-estate/issues/42)
and [#43](https://github.com/TusanHomichi/the-mortal-estate/issues/43), starting
at `6469c694e37aba4baddb3c7963ad100e61fc1298`. The owner authorized end-to-end
implementation, verification, and Git closeout. Preview deployment and paused
presentation adoption are outside this slice.

This History document owns the dated execution and proof receipts. The maintained fact
owners remain [server notes](../server-notes.md) for session teardown,
[client architecture](../client-architecture.md) for browser authority, and
[Workbench V0](../workbench-v0.md) for capture addressing. Browser implementation
and commands belong to [browser client](../browser-client.md).

## Required evidence

- Reproduce disconnected logout with retained diagnostics, fix its transition,
  and prove both teardown orders revoke the cookie and return HTTP 204.
- Render live and replayed authoritative frames in both browser engines; bind
  image, identity raster, sidecar, frame, and source digests; prove pointer and
  replay correspondence before restoring Workbench capture.
- Keep ordinary selection a file-reading operation, run affected checks and the
  full baseline, review the complete diff, and finish the authorized Git lifecycle.

## Findings

| Finding | Disposition and evidence |
| --- | --- |
| Disconnect can commit after logout opens its authentication snapshot | Forced on a scratch PostgreSQL database: HTTP 503 and `character-exit checkpoint read failed`, SQLSTATE `40001`. After moving the durable snapshot behind preparation, the same forced ordering returned 204. The gated live wire proof now includes the scheduling barrier. |
| Character replacement and expiry shared that ordering | Migrated with logout; expiry now uses the shared checkpoint persistence and publication helpers. Authentication is checked again in the durable transaction. |
| Browser schema consumption was absent | Rust codec compiled to WebAssembly with no new dependency. The complete shared wire corpus runs through the browser binding. Native corpus dispatch moved to the same decoder. |
| UUID generation features prevented a standalone WebAssembly build | Keep the version pin in the workspace; the server alone enables generation features. The protocol crate inherits serialization support. |
| Native test discovery did not recognize an `rlib`/`cdylib` harness | Runner maps the library harness to Cargo's `--lib`; its focused regression preserves Cargo's configured environment. |
| Cached capture bytes could outlive a changed file | Recheck capture digests before selection and image use; the fresh recording is independently bound. Mutants exercise both the saved packet and the cached capture. |
| Toolbar mouse-up overwrote a selected capture address | Gestures now require a press on the canvas. The served browser proof clicks the restored capture button, selects on the canvas, and clicks Record in both engines. |
| Session refresh re-enabled Capture before its image finished loading | Keep operation busy state in the shared UI state until completion; the browser proof requires the completion message when the button becomes available. |
| A relative Cargo target directory could be read from the web test directory | Resolve the codec artifact from the same checkout root used by its build. |
| Fresh capture could otherwise use another checkout's harness | Refuse that mismatch before any process starts; use the selected checkout's own tool. |

Document families: this completed execution record is History; the server,
client, browser, Workbench, settled-conclusions, and checkpoint documents are
Canonical; verification usage is Contract; the prior audit receipt is History
and changed only to repair its owner link. Each maintains the ownership stated
above rather than duplicating the implementation contract.

## Observed verification

- The first resolved fast run selected `portable, web`. All boundaries, docs,
  Python groups, 1,414 native tests across 35 harnesses, formatting, Clippy, and
  the Workbench demo passed. TypeScript caught a typed-array buffer mismatch;
  that run was a failure, and later web steps correctly did not run.
- After the buffer fix, typecheck and all 415 browser tests passed, including
  the shared corpus and neutral-target tests.
- The live wire proof passed connected logout, completed disconnect, and forced
  overlapping teardown with old-cookie rejection and unused-ticket removal.
- Initial native-WSS capture/replay runs passed in both Chromium and Firefox on
  the actual Intel/Mesa hardware renderer. Each engine checked 23,998
  pointer/raster samples across live and replay. The complete served Workbench UI passed the capture button, canvas selection,
  and Record in both engines. Saved-packet and cached-capture recording mutants
  were refused. The observed four-capture operation took 60.5 seconds.
- A standalone verified recording replay produced matching artifacts in both
  engines without PostgreSQL. Relative replay paths are resolved from the
  selected working root; outside-root captures are refused.
- The first full run stopped at a stale banner-copy assertion in the Workbench
  tests. Updated it to require the diagnostic-view label now shown by the UI.
- The second full run passed through PostgreSQL and live wire proof, then the
  browser UI completion assertion caught the early Capture re-enable. Fixed the
  shared busy state; the final rerun below owns completion of the baseline.
- Final command: `TME_PG_ADMIN_URL_FILE=<scratch-admin-file> python3
  tools/run_verification.py --scope full --keep-going --report-disk` — exit 0,
  **COMPLETE**, every selected step passed in 788.281 seconds. This includes all
  boundary/Python/native/web surfaces, six isolated PostgreSQL proof groups,
  live logout, both-browser Workbench UI capture, and the clean-copy gate.
  The working checkout proved the real private denylist; the deliberately
  private-free copy proved its synthetic mechanism and reported that distinction.
- The clean copy carried 1,010 files, built and tested successfully, and peaked
  at 6,446 MiB of build output. It used no local capture or private input.
- `CARGO_TARGET_DIR=target npm --prefix web test` also passed all 415 tests,
  proving relative target-directory resolution.
- Final diff and fact-class review repaired stale codec/capture assertions in
  their existing owners. Routing, relative links, and public-boundary checks
  passed. Every finding above is fixed in this slice.

GitHub's PR and issue records own the subsequent delivered revision and closure
state; this receipt records the implementation and locally observed proof.

No artwork, preview deployment, or paused-plan acceptance is claimed.
