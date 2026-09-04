#!/usr/bin/env python3
"""Prove the authoritative pulse is visible, by photographing it moving.

Charter item 3 asks for one clock, felt. `tools/run_client_live_proof.py`
already establishes that the beat a real client observes against a real server
is the ruled 3.0 seconds. What it cannot establish is that anything in the
picture says so, because it runs headless and there is no picture.

This driver uses the shared live-server harness with a corpus fixture, drives the shipped
`ClientRoot.tscn` under a virtual display, and captures the window at three
known points inside **one** beat. Then it judges the result:

* the samples belong to one round of logical time — three pictures spread over
  two beats would show a meter advancing without proving it ever reset;
* the fill strictly increases across them, and spans enough of the beat that a
  frozen or jittering meter could not produce it;
* the beat the client measured for itself is the ruled cadence — the client is
  never told the pulse, it observes it, so this is a real agreement between two
  independent statements of the same fact rather than a constant checked
  against itself;
* every sample's meter text agrees with the frame's own readiness, because a
  meter that decided readiness locally is the exact defect ruling D5 forbids.

Usage:

    tools/run_pulse_capture.py \\
        --admin-url-file <postgres superuser url file> \\
        --godot <pinned godot binary> \\
        --output <directory>
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from live_server_harness import (  # noqa: E402
    GODOT_VERSION,
    LiveServer,
    ProofError,
    World,
    emit_client_output,
    read_admin_url,
    resolve_godot,
)
from run_client_live_proof import (  # noqa: E402
    PULSE_TOLERANCE_MSEC,
    RULED_PULSE_MSEC,
)

SUCCESS_SENTINEL = "TME_PULSE_CAPTURE_OK"
CLIENT_SCRIPT = "res://tests/pulse_capture.gd"
MANIFEST_NAME = "pulse.json"
WINDOW = (1024, 768)
XVFB_SCREEN = "1280x1024x24"

#: This capture uses the first-land corpus fixture; the sign-in live proof uses
#: the authored identity-proof land. Both exercise the same server scheduler.
FIRST_LAND = World(
    world_template="content/test-corpus/world_templates/first_land_structure.json",
    simulation_seed="content/test-corpus/simulation_seeds/first_land_structure.json",
)

#: How far apart the first and last sampled fills must be. The samples are
#: requested at 0.15 and 0.85 of the beat, so a run that lands inside this is a
#: meter that moved; a run that does not is a meter that stalled, or a host so
#: loaded that the pictures cannot support the claim either way.
MINIMUM_FILL_SPREAD = 0.30

#: The smallest step between consecutive samples that counts as a distinct fill
#: state rather than the same one photographed twice.
MINIMUM_FILL_STEP = 0.05

#: The three files a capture is, all of which have to be on disk for the sample
#: to mean anything.
CAPTURE_FILES = ("capture.png", "capture.identity.pgm", "capture.sidecar.json")


def capture(arguments: argparse.Namespace) -> int:
    godot = resolve_godot(arguments.godot)
    if shutil.which("xvfb-run") is None:
        raise ProofError(
            "xvfb-run is required: a capture needs a real display, and Godot's "
            "headless display driver produces no viewport image"
        )
    output = Path(arguments.output).resolve()
    output.mkdir(parents=True, exist_ok=True)

    with LiveServer(
        read_admin_url(arguments.admin_url_file), FIRST_LAND, keep=arguments.keep
    ) as server:
        print("--- client ---", flush=True)
        client = server.run_client(
            godot,
            CLIENT_SCRIPT,
            extra_environment={"TME_CAPTURE_OUTPUT": str(output)},
            timeout=arguments.timeout,
            display=["xvfb-run", "-a", "--server-args", f"-screen 0 {XVFB_SCREEN}"],
            window=WINDOW,
        )
        emit_client_output(client)
        if client.returncode != 0 or SUCCESS_SENTINEL not in client.stdout:
            raise ProofError(f"the pulse capture failed with status {client.returncode}")
        print("--- server log tail ---")
        print(server.log_tail())

    manifest_path = output / MANIFEST_NAME
    if not manifest_path.is_file():
        raise ProofError(f"the client wrote no {MANIFEST_NAME}")
    print("--- the beat, as captured ---")
    check_manifest(json.loads(manifest_path.read_text(encoding="utf-8")), output)
    print(SUCCESS_SENTINEL)
    return 0


def check_manifest(manifest: dict, output: Path) -> None:
    """Judge the captured beat. Raises `ProofError` naming every failure."""
    samples = manifest.get("samples", [])
    if len(samples) < 3:
        raise ProofError(
            f"the run captured {len(samples)} sample(s); at least three inside one "
            "beat are needed to show the meter advancing and resetting"
        )

    failures: list[str] = []
    span = float(manifest.get("measured_span_msec", 0))
    drift = span - RULED_PULSE_MSEC
    verdict = "ok" if abs(drift) <= PULSE_TOLERANCE_MSEC else "OUT OF BAND"
    print(
        f"the client measured its own beat at {span:.0f} ms "
        f"({drift:+.0f} ms from the ruled {RULED_PULSE_MSEC:.0f} ms) {verdict}"
    )
    if abs(drift) > PULSE_TOLERANCE_MSEC:
        failures.append(
            f"the client measured a {span:.0f} ms beat, outside "
            f"{RULED_PULSE_MSEC:.0f} +/- {PULSE_TOLERANCE_MSEC:.0f} ms"
        )

    rounds = {str(sample.get("logical_time", "")) for sample in samples}
    if len(rounds) != 1:
        failures.append(
            "the samples span logical rounds "
            + ", ".join(f"T{value}" for value in sorted(rounds))
            + "; one beat's advance cannot be read across a beat boundary"
        )

    previous = None
    for sample in samples:
        fill = float(sample.get("fill", 0.0))
        print(
            f"sample {sample.get('index')}: T{sample.get('logical_time')} "
            f"fill {fill:.2f} :: {sample.get('meter_text', '')}"
        )
        failures.extend(_check_files(sample, output))
        failures.extend(_check_agreement(sample))
        if previous is not None and fill - previous < MINIMUM_FILL_STEP:
            failures.append(
                f"sample {sample.get('index')} filled to {fill:.2f} after "
                f"{previous:.2f}; the meter did not reach a distinct state"
            )
        previous = fill

    spread = float(samples[-1].get("fill", 0.0)) - float(samples[0].get("fill", 0.0))
    print(f"the meter advanced {spread:.2f} of a beat across the samples")
    if spread < MINIMUM_FILL_SPREAD:
        failures.append(
            f"the meter advanced {spread:.2f} of a beat between the first and last "
            f"sample; at least {MINIMUM_FILL_SPREAD:.2f} is needed to call it visible"
        )

    if failures:
        raise ProofError("the captured beat does not show the pulse: " + "; ".join(failures))
    print(f"{len(samples)} captures show the beat at {len(samples)} distinct fills, inside one round")


def _check_files(sample: dict, output: Path) -> list[str]:
    directory = Path(str(sample.get("directory", "")))
    if not directory.is_absolute():
        directory = output / directory
    missing = [name for name in CAPTURE_FILES if not (directory / name).is_file()]
    if missing:
        return [
            f"sample {sample.get('index')} is missing " + ", ".join(missing)
        ]
    return []


def _check_agreement(sample: dict) -> list[str]:
    """The picture's own claims, checked against the frame it was taken under.

    The meter is allowed to interpolate a fill and nothing else. Readiness is
    the frame's `can_act`, the wait is the frame's arithmetic, and the words on
    the meter have to say the same thing as both.
    """
    failures = []
    index = sample.get("index")
    text = str(sample.get("meter_text", ""))
    can_act = bool(sample.get("can_act", False))
    if can_act and "◆ Ready" not in text:
        failures.append(f"sample {index} was ready but the meter did not say so: {text!r}")
    if not can_act and "◇ Ready in" not in text:
        failures.append(f"sample {index} was waiting but the meter did not say so: {text!r}")
    if can_act and int(sample.get("beats_until_ready", -1)) != 0:
        failures.append(
            f"sample {index} claims readiness and a wait of "
            f"{sample.get('beats_until_ready')} beats at the same time"
        )
    if not bool(sample.get("measured", False)):
        failures.append(f"sample {index} drew a fill without having measured a beat")
    if not 0.0 <= float(sample.get("fill", -1.0)) <= 1.0:
        failures.append(f"sample {index} reports a fill of {sample.get('fill')}")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--admin-url-file",
        required=True,
        help="file holding a PostgreSQL superuser URL used only to create and drop the scratch database",
    )
    parser.add_argument(
        "--godot",
        default=os.environ.get("TME_GODOT", ""),
        help="Godot binary, pinned to " + GODOT_VERSION,
    )
    parser.add_argument(
        "--output",
        default=os.environ.get("TME_CAPTURE_OUTPUT", ""),
        help="directory to write the pulse captures into",
    )
    parser.add_argument("--timeout", type=float, default=300.0, help="seconds allowed for the client run")
    parser.add_argument("--keep", action="store_true", help="keep the scratch database and run directory")
    arguments = parser.parse_args()
    if not arguments.godot:
        parser.error("--godot or TME_GODOT must name the pinned Godot binary")
    if not arguments.output:
        parser.error("--output or TME_CAPTURE_OUTPUT must name a directory to write into")
    try:
        return capture(arguments)
    except ProofError as error:
        print(f"pulse capture failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
