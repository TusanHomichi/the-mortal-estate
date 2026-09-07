import type { Group, PointLight } from "three";

/** One authored room owns indirect fill and the bounded practical shadow budget. */
export interface InteriorLighting {
  indirect_intensity: number;
  shadow_count: number;
  source_decay: number;
  source_color: string;
  shadow_radius: number;
  shadow_strength: number;
}

export function parseInteriorLighting(value: unknown): InteriorLighting {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid interior lighting profile");
  const row = value as Record<string, unknown>;
  if (Object.keys(row).length !== 6 || !["indirect_intensity", "shadow_count", "source_decay", "source_color", "shadow_radius", "shadow_strength"].every(key => Object.hasOwn(row, key)) ||
      typeof row.indirect_intensity !== "number" || !Number.isFinite(row.indirect_intensity) ||
      row.indirect_intensity < 0 || row.indirect_intensity > .5 ||
      typeof row.shadow_count !== "number" || !Number.isInteger(row.shadow_count) || row.shadow_count < 0 || row.shadow_count > 2 ||
      typeof row.source_decay !== "number" || !Number.isFinite(row.source_decay) || row.source_decay < 1 || row.source_decay > 2 ||
      typeof row.source_color !== "string" || !/^#[0-9a-fA-F]{6}$/.test(row.source_color) ||
      typeof row.shadow_radius !== "number" || !Number.isFinite(row.shadow_radius) || row.shadow_radius < 0 || row.shadow_radius > 4 ||
      typeof row.shadow_strength !== "number" || !Number.isFinite(row.shadow_strength) || row.shadow_strength < 0 || row.shadow_strength > 1) {
    throw new Error("invalid interior lighting profile");
  }
  return { indirect_intensity: row.indirect_intensity, shadow_count: row.shadow_count,
    source_decay: row.source_decay, source_color: row.source_color,
    shadow_radius: row.shadow_radius, shadow_strength: row.shadow_strength };
}

export function resolveInteriorLighting(roots: readonly Group[]): InteriorLighting | null {
  let profile: InteriorLighting | null = null;
  for (const root of roots) root.traverse(node => {
    if (!Object.hasOwn(node.userData, "tme_interior_lighting")) return;
    const candidate = parseInteriorLighting(node.userData.tme_interior_lighting);
    if (profile) throw new Error("multiple interior lighting owners in one space");
    profile = candidate;
  });
  return profile;
}

/** Stable strongest-source selection avoids a shadow map for every small candle. */
export function configurePracticalShadows(lights: readonly PointLight[], profile: InteriorLighting | null): void {
  if (!profile) return;
  const selected = [...lights].filter(light => light.intensity > 0)
    .sort((a, b) => b.intensity - a.intensity).slice(0, profile.shadow_count);
  for (const light of selected) {
    light.castShadow = true;
    light.shadow.mapSize.set(256, 256);
    light.shadow.camera.near = .04;
    light.shadow.camera.far = light.distance;
    light.shadow.bias = -.0003;
    light.shadow.normalBias = .018;
    light.shadow.radius = profile.shadow_radius;
    light.shadow.intensity = profile.shadow_strength;
  }
}
