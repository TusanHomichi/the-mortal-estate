import { describe, expect, it } from "vitest";
import { Group, InstancedMesh, Matrix4, Vector2, Vector3, Frustum, OrthographicCamera } from "three";
import { pathCover } from "../src/space/groundCover";
import { addGrassCover } from "../src/space/grassCover";
import { createTerrainSurface } from "../src/terrainSurface";
import type { FeelSpace } from "../src/feelTypes";

describe("natural ground margins", () => {
  it("breaks up mixed margins without changing clear paths or uninterrupted turf", () => {
    const field = { height: .07, land: 1, meadow: 1, lane: .5 };
    const values = new Set<number>();
    for (let x = -2; x < 2; x += .125) {
      values.add(pathCover(field, x, .7));
      expect(pathCover({ ...field, lane: 0 }, x, .7)).toBe(0);
      expect(pathCover({ ...field, lane: 1 }, x, .7)).toBe(1);
    }
    expect(Math.min(...values)).toBeLessThan(.45);
    expect(Math.max(...values)).toBeGreaterThan(.55);
    expect(field).toEqual({ height: .07, land: 1, meadow: 1, lane: .5 });
  });

  it("roots deterministic cover on rendered land and keeps worn path centres clear", () => {
    const materials = ["water", "grass", "grass", "lane", "meadow", "meadow", "stone"];
    const space: FeelSpace = { grid_extents: { i: 7, j: 5 },
      cells: Array.from({ length: 35 }, (_, n) => ({ i: n % 7, j: Math.floor(n / 7),
        material: materials[n % 7]!, walkable: n % 7 > 0 })),
      structures: [], props: [], wall_runs: [], roofs: [], fixtures: [], portals: [],
      weather: true, light_sources: { lantern_glass: null, candles: [] } };
    const surface = createTerrainSurface(space);
    const build = () => {
      const group = new Group();
      const cover = addGrassCover(group, space, surface, { elapsed: { value: 0 },
        windDirection: { value: new Vector2(1, 0) }, windStrength: { value: 1 } });
      return { cover, group, meshes: (group.getObjectByName("GrassCover") as Group).children as InstancedMesh[] };
    };
    const a = build(), b = build();
    expect(a.meshes.map(m => m.instanceMatrix.array)).toEqual(b.meshes.map(m => m.instanceMatrix.array));
    expect(a.meshes.reduce((count, m) => count + m.count, 0)).toBe(a.cover.count);
    expect(a.meshes.length).toBeGreaterThan(1);
    expect(a.cover.count).toBeGreaterThan(0);
    let short = 0, tall = 0;
    const matrix = new Matrix4(), position = new Vector3();
    for (const mesh of a.meshes) for (let n = 0; n < mesh.count; n++) {
      mesh.getMatrixAt(n, matrix); position.setFromMatrixPosition(matrix);
      const sample = surface.sample(position.x, position.z);
      expect(position.y).toBeCloseTo(sample.height, 5);
      expect(sample.land).toBeGreaterThanOrEqual(.83999);
      expect(pathCover(sample, position.x, position.z)).toBeLessThanOrEqual(.53001);
      expect(position.x).toBeLessThan(5.5);
      const h = new Vector3().setFromMatrixScale(matrix).y;
      if (position.x > 1.5 && position.x < 2) { expect(h).toBeLessThan(.14); short++; }
      if (position.x > 4.5 && position.x < 5) { expect(h).toBeGreaterThan(.27); tall++; }
    }
    expect(short).toBeGreaterThan(0); expect(tall).toBeGreaterThan(0);
    // Batches can be culled individually with a conservative motion envelope.
    const camera = new OrthographicCamera(-.1,.1,.1,-.1,.1,20);
    camera.position.set(.8,10,1); camera.lookAt(.8,0,1); camera.updateMatrixWorld();
    const frustum = new Frustum().setFromProjectionMatrix(new Matrix4().multiplyMatrices(camera.projectionMatrix,camera.matrixWorldInverse));
    a.group.updateMatrixWorld(true);
    const visible = a.meshes.filter(mesh => frustum.intersectsObject(mesh));
    expect(visible.length).toBeGreaterThan(0); expect(visible.length).toBeLessThan(a.meshes.length);
    for (const mesh of a.meshes) {
      const motionRadius = mesh.boundingSphere!.radius;
      mesh.computeBoundingSphere();
      expect(motionRadius - mesh.boundingSphere!.radius).toBeGreaterThan(.48);
    }
    for (const { cover, group } of [a, b]) {
      cover.dispose();
      const geometries = new Set<InstancedMesh["geometry"]>();
      const materials = new Set<InstancedMesh["material"]>();
      group.traverse(ob => { if (ob instanceof InstancedMesh) { geometries.add(ob.geometry); materials.add(ob.material); } });
      for (const g of geometries) g.dispose();
      for (const m of materials) if (!Array.isArray(m)) m.dispose();
    }
  });
});
