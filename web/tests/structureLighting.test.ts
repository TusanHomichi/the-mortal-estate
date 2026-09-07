import { describe, expect, it, vi } from "vitest";
import { Group, Object3D, PointLight } from "three";
import { createFeelCamera } from "../src/camera";
import { paletteFor } from "../src/space/palette";
import { createStructureAtmosphere, validateStructureAtmosphere } from "../src/space/structureAtmosphere";
import { parseInteriorLighting, resolveInteriorLighting } from "../src/space/structureLighting";

const profile = { indirect_intensity: .2, shadow_count: 1, source_decay: 1, source_color: "#ffe0b2",
  shadow_radius: 3, shadow_strength: .55 };
const room = () => {
  const root = new Group(); root.userData.tme_interior_lighting = { ...profile }; return root;
};

describe("authored practical room lighting", () => {
  it("removes the unanchored key and takes indirect fill from the single room owner", () => {
    const root = room();
    const authored = resolveInteriorLighting([root]);
    const palette = paletteFor(["day"], false, authored);
    expect(palette.keyIntensity).toBe(0);
    expect(palette.ambientIntensity).toBe(.2);
    expect(paletteFor(["night"], false, authored)).toEqual(palette);
    expect(() => paletteFor(["day"], true, authored)).toThrow(/outdoor/);
    expect(paletteFor(["day"], true).keyIntensity).toBeGreaterThan(0);
    expect(resolveInteriorLighting([new Group()])).toBeNull();
  });

  it("refuses competing owners and malformed profiles at asset decoding", () => {
    expect(() => resolveInteriorLighting([room(), room()])).toThrow(/multiple/);
    for (const value of [null, [], {}, { ...profile, extra: true },
      { ...profile, indirect_intensity: Infinity }, { ...profile, indirect_intensity: -.1 },
      { ...profile, indirect_intensity: .6 }, { ...profile, shadow_count: 1.5 }, { ...profile, shadow_count: 3 },
      { ...profile, source_decay: NaN }, { ...profile, source_decay: 0 }, { ...profile, source_decay: 3 },
      { ...profile, source_color: "white" }, { ...profile, source_color: "#fff" },
      { ...profile, shadow_radius: NaN }, { ...profile, shadow_radius: -1 }, { ...profile, shadow_radius: 5 },
      { ...profile, shadow_strength: Infinity }, { ...profile, shadow_strength: -.1 }, { ...profile, shadow_strength: 1.1 },
      { indirect_intensity: .2, shadow_count: 1, source_decay: 1, source_color: "#ffe0b2" },
      { indirect_intensity: .2, shadow_count: 1 }]) {
      const root = room(); root.userData.tme_interior_lighting = value;
      expect(() => validateStructureAtmosphere(root)).toThrow(/interior lighting/);
    }
    const dark = { ...profile, indirect_intensity: 0, shadow_count: 0 };
    expect(parseInteriorLighting(dark)).toEqual(dark);
  });

  it("lights authored anchors, limits shadows to active strong sources, and disposes them", () => {
    const root = room(); root.position.set(4, 0, 2);
    for (const [intensity, period] of [[2, "day"], [6, "day"], [8, "night"]] as const) {
      const anchor = new Object3D(); anchor.position.set(intensity, 1, 0);
      anchor.userData.tme_light = { periods: [period], intensity, distance: 5 };
      root.add(anchor);
    }
    const effect = createStructureAtmosphere(root, ["day"], createFeelCamera(768, 512, { i: 0, j: 0 }), profile);
    const lights = root.children.filter((node): node is PointLight => node instanceof PointLight);
    expect(lights.map(light => light.intensity)).toEqual([2, 6]);
    expect(lights.map(light => light.decay)).toEqual([1, 1]);
    expect(lights.map(light => light.color.getHexString())).toEqual(["ffe0b2", "ffe0b2"]);
    expect(lights.map(light => light.position.toArray())).toEqual([[2, 1, 0], [6, 1, 0]]);
    expect(lights.map(light => light.castShadow)).toEqual([false, true]);
    expect(lights[1]!.shadow.radius).toBe(3);
    expect(lights[1]!.shadow.intensity).toBe(.55);
    expect(lights[1]!.shadow.mapSize.toArray()).toEqual([256, 256]);
    const disposal = lights.map(light => vi.spyOn(light, "dispose"));
    effect.dispose();
    expect(root.children.some(node => node instanceof PointLight)).toBe(false);
    for (const spy of disposal) expect(spy).toHaveBeenCalledOnce();
  });
});
