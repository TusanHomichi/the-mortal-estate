import { describe, expect, it, vi } from "vitest";
import {
  BoxGeometry, DoubleSide, EqualDepth, Group, Mesh, MeshStandardMaterial,
  OrthographicCamera, ShaderMaterial, Texture, Vector2,
} from "three";
import { createSurfaceOcclusion } from "../src/space/surfaceOcclusion";
import { textureHitTest } from "../src/space/surfaceAlpha";

function fixture() {
  const scene = new Group();
  const actor = new Mesh(new BoxGeometry(.5, 1.6, .5), new MeshStandardMaterial());
  actor.position.y = .8;
  const root = new Group(); root.add(actor); scene.add(root);
  const camera = new OrthographicCamera(-5, 5, 5, -5, .1, 30);
  camera.position.set(0, 10, 10); camera.lookAt(0, 0, 0); camera.updateMatrixWorld();
  const material = new MeshStandardMaterial({ side: DoubleSide });
  const geometry = new BoxGeometry(2, 5, .1);
  const front = new Mesh(geometry, material); front.position.set(0, 2.5, 1);
  const back = new Mesh(geometry, material); back.position.set(0, 2.5, -2);
  const neighbour = new Mesh(geometry, material); neighbour.position.set(4, 2.5, 1);
  scene.add(front, back, neighbour);
  const controller = createSurfaceOcclusion([front, back, neighbour].map(mesh => ({ mesh })), root, camera);
  return { scene, root, front, back, neighbour, material, geometry, controller };
}

describe("foreground surface visibility", () => {
  it("fades only blocking geometry, isolates shared materials and restores after departure", () => {
    const f = fixture();
    f.controller.update(0); f.controller.update(.3);
    expect(f.controller.snapshot()).toEqual([{ name: "/", id: f.front.id, opacity: .4 }]);
    expect(f.front.material).not.toBe(f.material);
    expect(f.front.material.opacity).toBe(.4);
    expect(f.back.material).toBe(f.material); expect(f.neighbour.material).toBe(f.material);
    expect(f.material.opacity).toBe(1);
    f.root.position.x = -4;
    f.controller.update(.5); f.controller.update(.65);
    expect(f.front.material.opacity).toBeGreaterThan(.4);
    expect(f.front.material.opacity).toBeLessThan(1);
    f.root.position.x = 0;
    f.controller.update(.74);
    const beforeReverse = f.front.material.opacity;
    f.controller.update(.82);
    expect(f.front.material.opacity).toBeLessThan(beforeReverse);
    f.root.position.x = -4;
    f.controller.update(1); f.controller.update(1.31);
    expect(f.controller.snapshot()).toEqual([]);
    expect(f.front.material).toBe(f.material);
    expect(f.scene.children).toHaveLength(4);
    f.controller.dispose();
  });

  it("uses matching depth geometry to blend stacked surfaces once, retaining source resources", () => {
    const f = fixture();
    const disposeMaterial = vi.spyOn(f.material, "dispose");
    const disposeGeometry = vi.spyOn(f.geometry, "dispose");
    f.controller.update(0); f.controller.update(.3);
    const depth = f.scene.children.find(o => o.name.endsWith("/occlusion-depth")) as Mesh;
    expect(depth.geometry).toBe(f.front.geometry);
    expect(depth.matrix).toEqual(f.front.matrix);
    expect(depth.castShadow).toBe(false);
    expect((depth.material as MeshStandardMaterial).colorWrite).toBe(false);
    expect((depth.material as MeshStandardMaterial).depthWrite).toBe(true);
    expect(f.front.material.depthFunc).toBe(EqualDepth);
    expect(f.front.material.depthWrite).toBe(false);
    expect(depth.renderOrder).toBeLessThan(f.front.renderOrder);
    const colourDispose = vi.spyOn(f.front.material, "dispose");
    const depthDispose = vi.spyOn(depth.material as MeshStandardMaterial, "dispose");
    f.material.emissiveIntensity = 3;
    f.controller.update(.4);
    expect(f.front.material.emissiveIntensity).toBe(3);
    f.controller.dispose();
    expect(colourDispose).toHaveBeenCalledTimes(1); expect(depthDispose).toHaveBeenCalledTimes(1);
    expect(disposeMaterial).not.toHaveBeenCalled(); expect(disposeGeometry).not.toHaveBeenCalled();
    expect(f.front.material).toBe(f.material);
  });

  it("keeps custom lighting callbacks and live wind uniforms on temporary materials", () => {
    const f = fixture(); f.controller.dispose();
    const material = new ShaderMaterial({ uniforms: { elapsed: { value: 0 } },
      fragmentShader: "void main() { gl_FragColor = vec4(1.); }" });
    material.onBeforeCompile = () => {}; material.customProgramCacheKey = () => "test";
    (f.front as Mesh).material = material;
    // Use the same fixed camera as the foreground fixture.
    const camera = new OrthographicCamera(-5, 5, 5, -5, .1, 30);
    camera.position.set(0, 10, 10); camera.lookAt(0, 0, 0);
    const fade = createSurfaceOcclusion([{ mesh: f.front }], f.root, camera);
    fade.update(0); fade.update(.3);
    const faded = f.front.material as unknown as ShaderMaterial;
    expect(faded.onBeforeCompile).toBe(material.onBeforeCompile);
    expect(faded.customProgramCacheKey()).toBe("test");
    expect(faded.uniforms.elapsed).toBe(material.uniforms.elapsed);
    expect(faded.uniforms.surfaceOpacity!.value).toBe(.4);
    expect(faded.fragmentShader).toContain("surfaceMain(); gl_FragColor.a *= surfaceOpacity");
    fade.dispose();
  });

  it("does not select transparent card padding and accounts for UV orientation", () => {
    const texture = new Texture();
    const acceptsHit = textureHitTest({ texture, width: 2, height: 2,
      pixels: new Uint8ClampedArray([0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255]) });
    expect(acceptsHit({ uv: new Vector2(.1, .9) } as never)).toBe(true);
    expect(acceptsHit({ uv: new Vector2(.1, .1) } as never)).toBe(false);
    const f = fixture(); f.controller.dispose();
    const camera = new OrthographicCamera(-5, 5, 5, -5, .1, 30);
    camera.position.set(0, 10, 10); camera.lookAt(0, 0, 0);
    const fade = createSurfaceOcclusion([{ mesh: f.front, acceptsHit: () => false }], f.root, camera);
    fade.update(0); fade.update(.3);
    expect(fade.snapshot()).toEqual([]); expect(f.front.material).toBe(f.material);
    fade.dispose();
  });
});
