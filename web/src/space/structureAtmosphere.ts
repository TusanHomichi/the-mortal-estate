import {
  Color, Group, InstancedBufferAttribute, InstancedMesh, Material, Mesh, MeshStandardMaterial,
  Object3D, OrthographicCamera, PlaneGeometry, PointLight, ShaderMaterial,
} from "three";
import { lightingPeriod, windPresetSettings, type Preset } from "../presets";
import { configurePracticalShadows, parseInteriorLighting, type InteriorLighting } from "./structureLighting";

type Period = ReturnType<typeof lightingPeriod>;
interface Timed { periods: Period[] }
interface Practical extends Timed { intensity: number }
interface Smoke extends Timed { rise: number; radius: number }
interface Lamp extends Practical { distance: number }
const PERIODS = ["day", "dusk", "night"];

function timed(value: unknown, fields: Record<string, readonly [number, number]>): Timed & Record<string, number> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("invalid structure atmosphere metadata");
  const row = value as Record<string, unknown>;
  const names = ["periods", ...Object.keys(fields)];
  if (Object.keys(row).length !== names.length || Object.keys(row).some(k => !names.includes(k)) ||
      !Array.isArray(row.periods) || row.periods.length === 0 ||
      row.periods.some(p => typeof p !== "string" || !PERIODS.includes(p)) ||
      new Set(row.periods).size !== row.periods.length) throw new Error("invalid structure atmosphere schedule");
  for (const [name, [min, max]] of Object.entries(fields)) {
    const n = row[name];
    if (typeof n !== "number" || !Number.isFinite(n) || n < min || n > max) throw new Error(`invalid structure atmosphere ${name}`);
  }
  return row as unknown as Timed & Record<string, number>;
}
const practical = (v: unknown): Practical => timed(v, { intensity: [0, 4] }) as unknown as Practical;
const smoke = (v: unknown): Smoke => timed(v, { rise: [0.2, 4], radius: [0.05, 0.8] }) as unknown as Smoke;
const lamp = (v: unknown): Lamp => timed(v, { intensity: [0, 8], distance: [0.1, 6] }) as unknown as Lamp;

/** These names are an explicit GLB extras contract, never inferred from mesh names. */
export function validateStructureAtmosphere(root: Group): void {
  root.traverse(node => {
    if (Object.hasOwn(node.userData, "tme_interior_lighting")) parseInteriorLighting(node.userData.tme_interior_lighting);
    if (Object.hasOwn(node.userData, "tme_smoke")) smoke(node.userData.tme_smoke);
    if (Object.hasOwn(node.userData, "tme_light")) lamp(node.userData.tme_light);
    if (!(node instanceof Mesh)) return;
    for (const material of Array.isArray(node.material) ? node.material : [node.material]) {
      if (!Object.hasOwn(material.userData, "tme_practical")) continue;
      practical(material.userData.tme_practical);
      if (!(material instanceof MeshStandardMaterial)) throw new Error("structure practical requires a lit standard material");
    }
  });
}

const PUFFS = 12;
function chimneyPuffs(spec: Smoke, presets: readonly Preset[], camera: OrthographicCamera) {
  const geometry = new PlaneGeometry(1, 1);
  const material = new ShaderMaterial({
    transparent: true, depthWrite: false,
    uniforms: { tint: { value: new Color("#9cabb1") } },
    vertexShader: `varying vec2 vUv; varying float vAlpha;
      attribute float puffAlpha;
      void main(){vUv=uv;vAlpha=puffAlpha;gl_Position=projectionMatrix*modelViewMatrix*instanceMatrix*vec4(position,1.);}`,
    fragmentShader: `uniform vec3 tint; varying vec2 vUv; varying float vAlpha;
      void main(){vec2 p=vUv*2.-1.;
        float edge=length(p)+.09*sin(p.x*8.+p.y*3.)+.06*sin(p.y*11.-p.x*5.);
        float a=(1.-smoothstep(.22,1.,edge))*vAlpha;
        gl_FragColor=vec4(tint,a);
        #include <tonemapping_fragment>
        #include <colorspace_fragment>
      }`,
  });
  const mesh = new InstancedMesh(geometry, material, PUFFS);
  mesh.name = "Chimney smoke";
  mesh.frustumCulled = false;
  mesh.renderOrder = 2;
  const dummy = new Object3D();
  const wind = windPresetSettings(presets, true);
  const alphas = new Float32Array(PUFFS);
  // Loaded lazily with the scene, not an external smoke texture.
  return { mesh, geometry, material, dummy, wind, alphas, spec, camera };
}


/** Per-space clones own changing emission; cached GLB materials remain immutable. */
export function createStructureAtmosphere(parent: Group, presets: readonly Preset[], camera: OrthographicCamera,
  interiorLighting: InteriorLighting | null = null) {
  const period = lightingPeriod(presets);
  const clones = new Map<Material, MeshStandardMaterial>();
  const emitters: ReturnType<typeof chimneyPuffs>[] = [];
  const lights: PointLight[] = [];
  parent.updateMatrixWorld(true);
  const nodes: Object3D[] = [];
  parent.traverse(node => nodes.push(node));
  for (const node of nodes) {
    if (node instanceof Mesh && node.userData.sharedStructure) {
      const replace = (source: Material): Material => {
        if (!Object.hasOwn(source.userData, "tme_practical")) return source;
        const spec = practical(source.userData.tme_practical);
        let clone = clones.get(source);
        if (!clone) {
          clone = (source as MeshStandardMaterial).clone();
          clone.emissiveIntensity = spec.periods.includes(period) ? spec.intensity : 0;
          clones.set(source, clone);
        }
        return clone;
      };
      node.material = Array.isArray(node.material) ? node.material.map(replace) : replace(node.material);
    }
    if (Object.hasOwn(node.userData, "tme_light")) {
      const spec = lamp(node.userData.tme_light);
      if (spec.periods.includes(period)) {
        const light = new PointLight(interiorLighting?.source_color ?? "#ffb76b", spec.intensity,
          spec.distance, interiorLighting?.source_decay ?? 2);
        light.position.copy(parent.worldToLocal(node.getWorldPosition(light.position)));
        parent.add(light); lights.push(light);
      }
    }
    if (Object.hasOwn(node.userData, "tme_smoke")) {
      const spec = smoke(node.userData.tme_smoke);
      if (!spec.periods.includes(period)) continue;
      const emitter = chimneyPuffs(spec, presets, camera);
      emitter.geometry.setAttribute("puffAlpha", new InstancedBufferAttribute(emitter.alphas, 1));
      emitter.mesh.position.copy(parent.worldToLocal(node.getWorldPosition(emitter.mesh.position)));
      parent.add(emitter.mesh); emitters.push(emitter);
    }
  }
  configurePracticalShadows(lights, interiorLighting);
  return {
    update(elapsed: number) {
      for (const [index, e] of emitters.entries()) {
        for (let n = 0; n < PUFFS; n += 1) {
          const age = (elapsed / 6 + n / PUFFS + index * 0.37) % 1;
          const swell = e.spec.radius * (0.25 + age * 2);
          e.dummy.position.set(
            age * age * e.wind.strength * e.wind.direction[0] + Math.sin(n * 2.4 + age * 5) * age * .08,
            age * e.spec.rise,
            age * age * e.wind.strength * e.wind.direction[1],
          );
          e.dummy.quaternion.copy(e.camera.quaternion);
          e.dummy.scale.set(swell, swell * 1.25, 1);
          e.dummy.updateMatrix(); e.mesh.setMatrixAt(n, e.dummy.matrix);
          e.alphas[n] = .16 * Math.min(1, age * 10) * (1 - age) ** 1.4;
        }
        e.mesh.instanceMatrix.needsUpdate = true;
        e.geometry.getAttribute("puffAlpha").needsUpdate = true;
      }
    },
    dispose() {
      for (const clone of clones.values()) clone.dispose();
      for (const e of emitters) { parent.remove(e.mesh); e.geometry.dispose(); e.material.dispose(); }
      for (const light of lights) { parent.remove(light); light.dispose(); }
    },
  };
}
