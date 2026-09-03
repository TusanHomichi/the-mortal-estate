import { describe, expect, it } from "vitest";
import { AnimationClip, Bone, BoxGeometry, Group, Mesh, MeshBasicMaterial, MeshPhysicalMaterial, MeshStandardMaterial, Object3D, Skeleton, SkinnedMesh, Texture, VectorKeyframeTrack } from "three";
import { applyFigurePalette, assertClipBinds, assertPaintableMaterials, createFigureInstance, disposeDecodedFigures, resolveFigureUrl } from "../src/space/figureRig";
import type { DecodedFigure } from "../src/space/figureRig";

describe("figure files resolve only against verified bytes", () => {
  const table = new Map([["figure.bin", "blob:figure-bin"], ["T_Base.png", "blob:base"]]);

  it("maps a glTF's relative name, with or without a path or query, to its verified blob", () => {
    expect(resolveFigureUrl("figure.bin", table)).toBe("blob:figure-bin");
    expect(resolveFigureUrl("/feel-assets/figure.bin?x=1", table)).toBe("blob:figure-bin");
    expect(resolveFigureUrl("T_Base.png", table)).toBe("blob:base");
  });

  it("refuses a name the figure does not carry", () => {
    expect(() => resolveFigureUrl("T_Other.png", table)).toThrow(/unlisted file: T_Other.png/);
    expect(() => resolveFigureUrl("https://example.invalid/figure.bin", table)).not.toThrow();
    expect(() => resolveFigureUrl("https://example.invalid/evil.bin", table)).toThrow(/unlisted file/);
  });
});

describe("the figure palette patch", () => {
  const palette: [number, number, number][] = [[13, 7, 7], [200, 180, 160]];

  it("inserts the palette uniform and the lookup at three's anchor", () => {
    const material = new MeshStandardMaterial();
    applyFigurePalette(material, palette, 0.25);
    const shader = { uniforms: {} as Record<string, unknown>, fragmentShader: "void main() {\n#include <opaque_fragment>\n}", vertexShader: "" };
    material.onBeforeCompile!(shader as never, {} as never);
    expect(shader.fragmentShader).toContain("uniform vec3 figurePalette[2];");
    expect(shader.fragmentShader).toContain("for (int i = 0; i < 2; i++)");
    expect(shader.fragmentShader).toContain("0.2500");
    expect(Object.keys(shader.uniforms)).toContain("figurePalette");
    expect(material.customProgramCacheKey()).toBe("figure-palette-2-0.25");
  });

  it("refuses to build if the anchor has moved", () => {
    const material = new MeshStandardMaterial();
    applyFigurePalette(material, palette, 0.25);
    const shader = { uniforms: {}, fragmentShader: "void main() {}", vertexShader: "" };
    expect(() => material.onBeforeCompile!(shader as never, {} as never)).toThrow(/anchor the figure palette patches/);
  });
});

describe("disposing decoded figures", () => {
  it("releases the sources' geometry, materials, and textures once", () => {
    const texture = new Texture();
    const material = new MeshStandardMaterial({ map: texture });
    const geometry = new BoxGeometry();
    const rig = new Group();
    rig.add(new Mesh(geometry, material));
    const part = new Group();
    part.add(new Mesh(geometry, material)); // shared, as cloned instances share them
    const disposed = { geometry: 0, material: 0, texture: 0 };
    geometry.dispose = () => { disposed.geometry += 1; };
    material.dispose = () => { disposed.material += 1; };
    texture.dispose = () => { disposed.texture += 1; };
    const figure: DecodedFigure = { name: "f", rig, parts: [part], clips: [], palette: [[0, 0, 0], [1, 1, 1]], rim: 0, idle: "x" };
    disposeDecodedFigures(new Map([["f", figure]]));
    expect(disposed).toEqual({ geometry: 2, material: 1, texture: 1 });
  });
});

describe("figure materials the palette can patch", () => {
  it("accepts standard and physical materials", () => {
    const root = new Group();
    root.add(new Mesh(new BoxGeometry(), new MeshStandardMaterial()));
    root.add(new Mesh(new BoxGeometry(), new MeshPhysicalMaterial()));
    expect(() => assertPaintableMaterials(root, "figure test rig")).not.toThrow();
  });

  it("refuses an unlit or otherwise unpatchable material rather than rendering it unpainted", () => {
    const root = new Group();
    const mesh = new Mesh(new BoxGeometry(), new MeshBasicMaterial());
    mesh.name = "Hood";
    root.add(mesh);
    expect(() => assertPaintableMaterials(root, "figure test part")).toThrow(/carries a MeshBasicMaterial on Hood; the figure palette patches only standard materials/);
  });
});

describe("a clip must bind to the rig it plays on", () => {
  const clipOn = (node: string) => new AnimationClip("Idle", 1, [new VectorKeyframeTrack(`${node}.position`, [0, 1], [0, 0, 0, 0, 0, 0])]);

  it("accepts a clip whose every track finds its node", () => {
    const rig = new Group();
    const hips = new Object3D();
    hips.name = "Hips";
    rig.add(hips);
    expect(() => assertClipBinds(clipOn("Hips"), rig, "figure test rig")).not.toThrow();
  });

  it("refuses a clip that targets a bone the rig does not have, naming it", () => {
    const rig = new Group();
    expect(() => assertClipBinds(clipOn("Tail"), rig, "figure test part")).toThrow(/cannot play Idle: no node named Tail/);
  });
});

describe("an instance releases its cloned skeletons", () => {
  it("disposes every skinned mesh's skeleton on dispose", () => {
    const bone = new Bone();
    bone.name = "Hips";
    const skinned = new SkinnedMesh(new BoxGeometry(), new MeshStandardMaterial());
    skinned.add(bone);
    skinned.bind(new Skeleton([bone]));
    const rig = new Group();
    rig.add(skinned);
    const disposed: Skeleton[] = [];
    const original = Skeleton.prototype.dispose;
    Skeleton.prototype.dispose = function (this: Skeleton) { disposed.push(this); };
    try {
      const clip = new AnimationClip("Idle", 1, [new VectorKeyframeTrack("Hips.position", [0, 1], [0, 0, 0, 0, 0, 0])]);
      const instance = createFigureInstance({ name: "f", rig, parts: [], clips: [clip], palette: [[0, 0, 0], [1, 1, 1]], rim: 0, idle: "Idle" }, { i: 0, j: 0 }, 1);
      instance.dispose();
      expect(disposed).toHaveLength(1);
      expect(disposed[0]).not.toBe(skinned.skeleton); // the clone's, not the source's
    } finally {
      Skeleton.prototype.dispose = original;
    }
  });
});
