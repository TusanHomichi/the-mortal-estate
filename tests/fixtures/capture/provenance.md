---
last_updated: 2026-09-05
revision: 2
status: Paired captures freshly recorded for observer frame contract 8 and individual cooldown timing.
public_safe: true
summary: What produced the recorded frame and the two captures here, why they are tracked rather than generated at test time, and the exact commands that re-record them.
---

# Capture fixtures

Three artifacts, all produced by real runs, all tracked so that a clean clone can
prove the capture path with no client binary, no display, and no database.

| Path | What it is |
| --- | --- |
| `fixture_land_frame.json` | one authoritative frame, exactly as the real server sent it to the real client over the authoring fixture's land |
| `fixture-route/` | a capture of that frame taken by the ordinary route: the client alone, replaying the recorded frame |
| `live-route/` | a capture taken by the accuracy reference: the shipped client shell, admitted to a real server, photographing the frame it was actually sent |

Each capture directory holds the three files the presenter writes together —
`capture.png`, `capture.identity.pgm`, `capture.sidecar.json` — and each sidecar
names the digests of the other two. They are only meaningful as a set.

## Why both routes are tracked

The two captures are of **the same frame in the same land, framed differently**.
The live one was taken inside the world shell, so its lattice is inset around the
HUD; the fixture one has the window to itself. The sidecars record each
route's current square pitch. Different pixels, identical addresses — which is exactly the
claim acceptance criterion 2 makes, and it cannot be proven with one capture.

`tests/test_capture_addressing.py` replays the same world region through both
rasters and requires the same cells and the same semantic identities to come out.

## Why tracked rather than generated

A capture is a real client run: it needs the pinned client binary and a display.
Making the standing suite depend on either would mean the suite either skips its
own subject on most machines or, worse, passes without exercising it. Tracked
real artifacts let a clean clone prove the whole reading, addressing, staleness,
and parity path from files alone — the D6 rule, applied to captures.

What a clean clone therefore cannot prove is that a **freshly taken** capture
still matches these. That is what
`tests/test_capture_addressing.py::AFreshCaptureMatchesTheTrackedOne` is for; it
skips honestly, naming what is missing, when the client binary or the virtual
display is absent.

## Re-recording

Re-record when the frame contract changes, when the presenter's geometry changes,
or when the authoring fixture is recompiled.

The recorded frame and the live capture come from one run of the accuracy
reference:

```bash
tools/run_fixture_land_capture.py \
    --admin-url-file <postgres superuser url file> \
    --godot <pinned client binary> \
    --output tests/fixtures/capture/live-route \
    --record-frame tests/fixtures/capture/fixture_land_frame.json
```

The ordinary-route capture then replays the frame that run recorded:

```bash
cd client && TME_CAPTURE_FRAME=../tests/fixtures/capture/fixture_land_frame.json \
    TME_CAPTURE_OUTPUT=../tests/fixtures/capture/fixture-route \
    xvfb-run -a --server-args="-screen 0 1280x1024x24" \
    <pinned client binary> --path . --resolution 1024x768 \
    -s res://tests/capture_fixture_frame.gd
```

Record the live one **first**: the ordinary route replays the frame the live route
captured, and re-recording them the other way round would pair a new capture with
an old frame.

## What these carry no authority over

Nothing. They are proof material. No runtime reads them, no content references
them, and the land they show is the tracked authoring fixture, whose own
authority lives in `content/authoring-fixture/`.

## September 5 re-recording

Both routes and their authoritative frame were re-recorded after the observer
contract changed to version 8 with millisecond timestamps. The live capture was
taken first through the real server; the fixture capture replays that exact
recording. The diagnostic image, identity raster, and sidecar were replaced as
one set for each route. These remain proof fixtures, not accepted game artwork.
