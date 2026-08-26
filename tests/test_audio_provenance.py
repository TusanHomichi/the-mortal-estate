"""Close the carried client-audio set over bytes, provenance, and licenses."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
AUDIO_ROOT = ROOT / "client/presentation/audio"
MANIFEST = AUDIO_ROOT / "audio_manifest.generated.json"
PROVENANCE = AUDIO_ROOT / "asset-provenance.json"


class AudioProvenance(unittest.TestCase):
    def test_runtime_assets_have_exact_provenance_and_license_evidence(self) -> None:
        manifest = json.loads(MANIFEST.read_text())
        provenance = json.loads(PROVENANCE.read_text())

        self.assertEqual(provenance["schema_version"], 1)
        self.assertEqual(provenance["kind"], "client_audio_asset_provenance")
        self.assertEqual(provenance["license"], "CC0-1.0")

        sources = {source["id"]: source for source in provenance["sources"]}
        self.assertEqual(len(sources), len(provenance["sources"]))
        for source in sources.values():
            self.assertTrue(source["page"].startswith("https://"))
            evidence = AUDIO_ROOT / source["license_evidence"]
            self.assertTrue(evidence.is_file(), evidence)
            self.assertGreater(evidence.stat().st_size, 0, evidence)

        assets = {asset["path"]: asset for asset in provenance["assets"]}
        self.assertEqual(len(assets), len(provenance["assets"]))
        self.assertEqual(
            set(sources), {asset["source_id"] for asset in assets.values()}
        )
        carried_evidence = {
            path.relative_to(AUDIO_ROOT).as_posix()
            for path in (AUDIO_ROOT / "licenses").iterdir()
            if path.is_file()
        }
        self.assertEqual(
            carried_evidence,
            {source["license_evidence"] for source in sources.values()},
        )

        manifest_assets = {}
        for cue in manifest["cues"]:
            prefix = "res://presentation/audio/"
            self.assertTrue(cue["path"].startswith(prefix), cue["path"])
            relative = cue["path"][len(prefix) :]
            manifest_assets[relative] = cue

        carried = {
            path.relative_to(AUDIO_ROOT).as_posix()
            for path in (AUDIO_ROOT / "assets/generated").iterdir()
            if path.suffix in {".ogg", ".wav"}
        }
        self.assertEqual(set(assets), set(manifest_assets))
        self.assertEqual(set(assets), carried)

        for relative, asset in assets.items():
            self.assertIn(asset["source_id"], sources)
            self.assertTrue(asset["source_member"])
            cue = manifest_assets[relative]
            self.assertEqual(asset["sha256"], cue["sha256"])
            self.assertEqual(asset["byte_length"], cue["byte_length"])

            path = AUDIO_ROOT / relative
            body = path.read_bytes()
            self.assertEqual(len(body), asset["byte_length"], path)
            self.assertEqual(hashlib.sha256(body).hexdigest(), asset["sha256"], path)


if __name__ == "__main__":
    unittest.main()
