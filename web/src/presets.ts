export const KNOWN_PRESETS = ["night", "dusk", "rain", "fog", "wind"] as const;
export type Preset = (typeof KNOWN_PRESETS)[number];

const known = new Set<string>(KNOWN_PRESETS);

export function parsePresets(raw: string | null | undefined): Preset[] {
  const source = raw?.trim().toLowerCase() || "night";
  const parsed = source
    .split(",")
    .map((value) => value.trim())
    .filter((value): value is Preset => known.has(value));
  const unique: Preset[] = [...new Set(parsed)];
  if (unique.length === 0) return ["night"];
  return unique.sort();
}

export function presetsFromUrl(url: URL): Preset[] {
  return parsePresets(url.searchParams.get("preset"));
}

export interface WindPresetSettings {
  strength: number;
  gustPeriod: number;
  direction: readonly [number, number];
}

export function windPresetSettings(
  presets: readonly Preset[],
  weatherEnabled: boolean,
): WindPresetSettings {
  const base = 0.16;
  const wind = weatherEnabled && presets.includes("wind") ? 0.84 : 0;
  const rain = weatherEnabled && presets.includes("rain") ? 0.18 : 0;
  return {
    strength: base + wind + rain,
    gustPeriod: 9,
    direction: [0.86, 0.51],
  };
}
