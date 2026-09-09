import type { SurfaceSample } from "../terrainSurface";

/** Visual path wear shared by ground blending and rooted vegetation placement.
 * It never changes surface height, a material identity or a walking verdict.
 */
export function pathCover(sample: SurfaceSample, x: number, z: number): number {
  const margin = 4 * sample.lane * (1 - sample.lane);
  const irregularity = Math.sin(x * 2.1 + Math.sin(z * 3.3)) * .19
    + Math.sin(z * 7.3 - x * 4.7) * .07;
  return Math.max(0, Math.min(1, sample.lane + irregularity * margin));
}
