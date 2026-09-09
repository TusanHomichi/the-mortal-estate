/** Scenic bathymetry and exposure; never movement or swimming authority. */
export interface CoastalProfile {
  offshore_depth: number;
  shore_slope: number;
  zones: { centre: [number, number]; radius: [number, number]; depth: number; exposure: number }[];
}

export function parseCoastalProfile(value: unknown): CoastalProfile {
  const record = (v: unknown, keys: string[]): v is Record<string, unknown> =>
    typeof v === "object" && v !== null && !Array.isArray(v) &&
    Object.keys(v).length === keys.length && Object.keys(v).every(k => keys.includes(k));
  const number = (v: unknown, lo: number, hi: number): v is number =>
    typeof v === "number" && Number.isFinite(v) && v >= lo && v <= hi;
  const pair = (v: unknown, lo: number, hi: number): v is [number, number] =>
    Array.isArray(v) && v.length === 2 && v.every(n => number(n, lo, hi));
  if (!record(value, ["offshore_depth", "shore_slope", "zones"]) ||
    !number(value.offshore_depth, .2, 20) || !number(value.shore_slope, .02, 4) ||
    !Array.isArray(value.zones) || value.zones.length > 16) throw new Error("invalid coastal profile");
  const zones = value.zones.map(zone => {
    if (!record(zone, ["centre", "radius", "depth", "exposure"]) ||
      !pair(zone.centre, -200, 500) || !pair(zone.radius, .5, 100) ||
      !number(zone.depth, .08, 20) || !number(zone.exposure, 0, 1)) throw new Error("invalid coastal zone");
    return { centre: [...zone.centre] as [number, number], radius: [...zone.radius] as [number, number],
      depth: zone.depth, exposure: zone.exposure };
  });
  return { offshore_depth: value.offshore_depth, shore_slope: value.shore_slope, zones };
}

export function sampleCoastalProfile(profile: CoastalProfile | undefined, x: number, z: number, shore: number): { depth: number; exposure: number } {
  let depth = Math.min(profile?.offshore_depth ?? 3, .19 + Math.max(0, shore - .5) * (profile?.shore_slope ?? .45));
  let exposure = 1;
  // Ordered local overrides blend continuously; their order is authored data.
  for (const zone of profile?.zones ?? []) {
    const distance = Math.hypot((x - zone.centre[0]) / zone.radius[0], (z - zone.centre[1]) / zone.radius[1]);
    const t = Math.max(0, Math.min(1, (1 - distance) / .65));
    const weight = t * t * (3 - 2 * t);
    depth += (zone.depth - depth) * weight;
    exposure += (zone.exposure - exposure) * weight;
  }
  return { depth, exposure };
}
