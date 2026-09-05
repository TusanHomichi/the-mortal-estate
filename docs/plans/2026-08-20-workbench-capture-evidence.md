---
last_updated: 2026-09-05
revision: 1
status: Historical capture evidence extracted during Godot retirement. The commands and runtime below are retired, not current instructions.
public_safe: true
summary: Dated evidence for the original live/replay capture correspondence and measured costs.
---

# Historical Workbench capture evidence

Current operation and open browser integration live in [Workbench V0](../workbench-v0.md).

### The two routes

| | ordinary route | accuracy reference |
| --- | --- | --- |
| Command | `tools/workbench/capture_harness.py`, driven by the browser's *take a capture* button | `tools/run_fixture_land_capture.py` |
| What runs | the client alone, replaying one recorded authoritative frame | scratch database, schema, account, real server, TLS front, the shipped `ClientRoot.tscn` through sign-in and admission |
| Needs | the pinned client, a virtual display | all of the above plus a PostgreSQL superuser URL |
| Cost | **3.9 s** | **23.5 s** (14.0 s provisioning, 9.5 s client) |

The ordinary route is reproducible: re-running it over the same recorded frame
produced a byte-identical picture, raster and sidecar on this machine. The
raster and the sidecar are pure geometry and are reproducible anywhere; the
picture's bytes depend on the renderer, so the tracked currency check compares
the raster digest and the addresses rather than the PNG.

The frame the ordinary route replays is a **real server frame**, recorded by the
accuracy reference and tracked at
`tests/fixtures/capture/fixture_land_frame.json`. Replaying a recorded frame is
honest in a way synthesising one would not be: nothing invents a world, and a
fixture that drifted from the compiled land is caught by
`tests/test_capture_correspondence.py`.

Both routes serve **the fixture land** — the same compiled authoring fixture the
logical projection comes from. Criterion 2 is only meaningful over one land, and
the standing live proof's corpus land is a different one.

### What the two routes prove together

The two tracked captures show one frame of one land at two framings: the live one
inside the world shell, inset around the HUD, with 50-pixel squares; the ordinary
one with the window to itself, at 68. Every pixel differs. No address does —
`tests/test_capture_addressing.py::TheTwoCaptureRoutesResolveIdentically`.

### Decision 7 — drive the cheap route; keep the expensive one as the reference

**Ruled: the ordinary capture is the client alone replaying a recorded frame; the
real server route stays the accuracy reference, and both costs are published.**

The spec's warning was that pretending the expensive lane is fast is the one
option not available. It also assumed the expensive lane cost "minutes rather than
seconds". Measured, on this machine, it costs **23.5 seconds** — the predecessor's
credential- and display-gated integration lane was the expensive thing, and the
successor's provisioning is not it. That is worth recording precisely because the
spec's estimate is now wrong in the good direction.

Even so, the split stands, for reasons that are not about the stopwatch:

- The ordinary route needs no database, no superuser credential, no account, and
  no network. It is **3.9 s** and it can run on any machine with the client and a
  virtual display.
- The accuracy reference photographs a frame the real server actually sent, inside
  the real shell, and is the only thing that can catch the ordinary route drifting.
  It is run when the frame is re-recorded, and its capture is tracked so that the
  drift check runs in the standing suite without it.
- Neither is on the ordinary selection path. Selecting over a capture that already
  exists reads files: **0.33 ms** for a click.

Both timings are stated in the interface as well as here: the browser's capture
button says "runs the client · seconds, not milliseconds" before it is pressed,
and reports the elapsed time after.

## The cost of every loop, measured

Measured on the development machine on 2026-08-20, with software rendering
(llvmpipe). Medians of repeated runs; the capture figures are wall-clock for the
whole command.

| Operation | Cost |
| --- | --- |
| Logical click → resolved identities | 0.33 ms |
| Logical box over 16 cells → resolved identities | 0.40 ms |
| Capture click → resolved identities, through the raster | 0.33 ms |
| Capture box over 476×340 px (35 squares) | 5.3 ms |
| Capture box over the whole 1024×768 frame | 22 ms |
| Reading a capture: three files, three digests, header checks | 5.4 ms |
| Selection → packet written to disk (logical) | 1.2 ms |
| Selection → packet written to disk (capture) | 1.8 ms |
| **Capture request → capture on screen (ordinary route)** | **3.9 s** |
| **Capture request → capture on screen (accuracy reference)** | **23.5 s** |
| Boundary checks (`tools/run_checks.py`) | 14 s |
| The whole Python suite | 30 s |
| The whole client suite | 16 s |

Reproduce the two capture figures with:

```bash
TME_GODOT=<pinned client> python3 tools/workbench_demo.py     # prints the ordinary route's seconds
tools/run_fixture_land_capture.py --admin-url-file <url file> \
    --godot <pinned client> --output <directory>              # writes timings.json
```
