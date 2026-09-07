import { describe, expect, it, vi } from "vitest";
import { BoxGeometry, Group, Mesh, MeshStandardMaterial, Object3D, PointLight } from "three";
import { createFeelCamera } from "../src/camera";
import { createStructureAtmosphere, validateStructureAtmosphere } from "../src/space/structureAtmosphere";

function house() {
  const root = new Group();
  const material = new MeshStandardMaterial({ emissive: "#ffaa55", emissiveIntensity: 1 });
  material.userData.tme_practical = { periods: ["dusk"], intensity: .8 };
  const pane = new Mesh(new BoxGeometry(), material);
  pane.userData.sharedStructure = true; root.add(pane);
  const chimney = new Object3D(); chimney.position.set(3, 4, 5);
  chimney.userData.tme_smoke = { periods: ["dusk"], rise: 2, radius: .4 };
  chimney.userData.tme_light = { periods: ["dusk"], intensity: 1.5, distance: 2 };
  root.add(chimney);
  return { root, material, pane, chimney };
}
const camera = () => createFeelCamera(1280, 800, { i: 0, j: 0 });

describe("authored house atmosphere", () => {
  it("keeps a day instance dark without changing the shared dusk source", () => {
    const h = house(); validateStructureAtmosphere(h.root);
    const effect = createStructureAtmosphere(h.root, ["day"], camera());
    expect(h.pane.material.emissiveIntensity).toBe(0);
    expect(h.material.emissiveIntensity).toBe(1);
    expect(h.root.children).toHaveLength(2);
    const dispose = vi.spyOn(h.pane.material, "dispose");
    effect.dispose(); expect(dispose).toHaveBeenCalledOnce();
  });
  it("places active smoke and light at authored anchors and advances finite particles", () => {
    const h = house();
    const effect = createStructureAtmosphere(h.root, ["dusk", "wind"], camera());
    expect(h.pane.material.emissiveIntensity).toBe(.8);
    const light = h.root.children.find(n => n instanceof PointLight)!;
    const smoke = h.root.getObjectByName("Chimney smoke") as Mesh;
    expect(light.position.toArray()).toEqual([3, 4, 5]);
    expect(smoke.position.toArray()).toEqual([3, 4, 5]);
    effect.update(0); const before = [...smoke.geometry.getAttribute("puffAlpha").array];
    effect.update(2); const after = [...smoke.geometry.getAttribute("puffAlpha").array];
    expect(after).not.toEqual(before); expect(after.every(Number.isFinite)).toBe(true);
    effect.dispose(); expect(h.root.children).toHaveLength(2);
  });
  it("refuses bad schedules and unbounded emitters even when inactive", () => {
    for (const metadata of [
      { periods: ["midnight"], rise: 2, radius: .4 },
      { periods: ["day", "day"], rise: 2, radius: .4 },
      { periods: ["day"], rise: Infinity, radius: .4 },
      { periods: ["day"], rise: 2, radius: 7 },
      { periods: ["day"], rise: 2, radius: .4, extra: true },
    ]) {
      const h = house(); h.chimney.userData.tme_smoke = metadata;
      expect(() => validateStructureAtmosphere(h.root)).toThrow(/structure atmosphere/);
    }
  });
});
