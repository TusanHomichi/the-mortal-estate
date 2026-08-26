---
last_updated: 2026-08-27
revision: 2
status: Clean public-source audit repair; records the source, license, and digest of every tracked audio byte.
public_safe: true
summary: Per-asset provenance and CC0 license evidence for the ten audio files projected into the client.
---

# Client audio provenance

The `assets/generated/` name describes a generated **client projection**. These
ten files are not AI-generated. Nine are selected members of human-authored
Kenney foley packs and one is an individually verified human vocal recording
from Freesound. The projection copied the selected encoded bytes without
transcoding; the checked-in client files still match the recorded digests.

No runtime or build step downloads, trims, normalizes, resamples, or reads an
authoring/private root. The client manifest binds each projected file by digest
and byte length.

## Source and license transactions

- **Kenney RPG Audio:** official [pack page](https://www.kenney.nl/assets/rpg-audio)
  and archive; the in-pack notice is preserved as
  [`licenses/kenney-rpg-audio-cc0.txt`](licenses/kenney-rpg-audio-cc0.txt).
  Archive SHA-256:
  `6dbeaf8544da958d8f2adcb4a4a4b76c1ade34a05f8ab9edccd327da7375f38b`.
- **Kenney Impact Sounds:** official
  [pack page](https://www.kenney.nl/assets/impact-sounds) and archive; the
  in-pack notice is preserved as
  [`licenses/kenney-impact-sounds-cc0.txt`](licenses/kenney-impact-sounds-cc0.txt).
  Archive SHA-256:
  `029d734af1582474edf3a694d1b0cebc97c1c152f2f39fa34d4c2bafc5de77f8`.
- **Kenney Interface Sounds:** official
  [pack page](https://kenney.nl/assets/interface-sounds) and archive; the
  in-pack notice is preserved as
  [`licenses/kenney-interface-sounds-cc0.txt`](licenses/kenney-interface-sounds-cc0.txt).
  Archive SHA-256:
  `f2193d072726d6758a5f7871b2dcc54dcce0d5c35c6f0a62f92549b327c81232`.
- **Freesound sound 319253:** individually verified on its
  [source page](https://freesound.org/people/adharca/sounds/319253/) as the
  uploader's own non-verbal recording under CC0 1.0. The source-page and file
  evidence is preserved in
  [`licenses/freesound-319253-cc0.json`](licenses/freesound-319253-cc0.json).

All four sources permit commercial use under
[CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/). The notices and
attribution are preserved even where attribution is not mandatory.

## Selected pristine files

| Client file | Source/member | SHA-256 |
| --- | --- | --- |
| `combat_swing_01.ogg` | Kenney RPG Audio, `Audio/knifeSlice.ogg` | `4cd96dc630bed9840c15f1dd2306da2cc56a4da26a5d3f1a03c5a7265ac5e54f` |
| `combat_body_impact_01.ogg` | Kenney Impact Sounds, `Audio/impactPunch_heavy_000.ogg` | `b33a8f14068aec24ec69ba85e5e87fdc41228975f6a1a3e44a6e7d6fc3d9f8d8` |
| `combat_body_impact_02.ogg` | Kenney Impact Sounds, `Audio/impactPunch_heavy_001.ogg` | `f92f5cb6ba4ff2766497292ffd90865654317eeca976f5652e0708dbdcdc0dd9` |
| `combat_dry_result_01.ogg` | Kenney RPG Audio, `Audio/metalClick.ogg` | `9851a69d0c613e13bceef08060ecc4148f098ef487927cbebe270d642398a3b3` |
| `bow_release_01.ogg` | Kenney Interface Sounds, `Audio/pluck_001.ogg` | `be97ec4893a02d6eccfb678daa76c83e34cb2583b834ec2593d2641def739fa4` |
| `spell_chant_01.wav` | Freesound 319253, `319253__adharca__low-chant.wav` | `2042752db95720f22ba66a20977266f732b6213c609a9986d08c5e420185e87c` |
| `spell_release_01.ogg` | Kenney Interface Sounds, `Audio/pluck_002.ogg` | `c977fe249ff42d1c93a552b33abc13a8399df3879fa510475426e5c4bbac1da9` |
| `spell_impact_01.ogg` | Kenney Impact Sounds, `Audio/impactBell_heavy_000.ogg` | `94b8bb5f2d43ab65e4bcc32b28562416e9bc2c51d9fd4be1e333660ee52f977f` |
| `loot_stow_01.ogg` | Kenney RPG Audio, `Audio/handleSmallLeather2.ogg` | `a66f6db5918ad016c28a9f2768f127a7f39b5fa04481e5a3b4e78ebe2a1c282d` |
| `ui_reject_01.ogg` | Kenney Interface Sounds, `Audio/error_004.ogg` | `0b574cea597d96507e782ae9764f88482ce49f46e931e57054bf7150047f2d69` |

The byte lengths, playback roles, gains, and pitch bounds are carried in
[`audio_manifest.generated.json`](audio_manifest.generated.json). That manifest
is runtime routing. The machine-readable
[`asset-provenance.json`](asset-provenance.json), this document, and the adjacent
license files own provenance. `tests/test_audio_provenance.py` requires the
manifest, provenance inventory, carried media, digests, byte lengths, sources,
and license evidence to remain an exact closed set.
