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
