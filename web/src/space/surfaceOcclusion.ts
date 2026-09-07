import {
  Box3, EqualDepth, LessEqualDepth, Material, Mesh, MeshStandardMaterial,
  NoBlending, Object3D, OrthographicCamera, Raycaster, ShaderMaterial, Vector3,
} from "three";
import type { Intersection } from "three";

export const OCCLUDER_OPACITY = 0.4;
const TRANSITION_SECONDS = 0.3;
const SAMPLE_SECONDS = 0.075;
const RELEASE_SECONDS = 0.15;

export interface OccludingSurface {
  mesh: Mesh;
  /** Painted cards reject hits in transparent padding. Geometry needs no mask. */
  acceptsHit?: (hit: Intersection) => boolean;
}

interface SurfaceState extends OccludingSurface {
  bounds: Box3;
  original: Material | Material[];
  order: number;
  lastHit: number;
  amount: number;
  from: number;
  target: number;
  started: number;
  colours?: Material[];
  depth?: Mesh;
}

function materials(value: Material | Material[]): Material[] {
  return Array.isArray(value) ? value : [value];
}

function copyMaterial(source: Material): Material {
  const copy = source.clone();
  // Material.clone omits callbacks; card lighting and figure-independent
  // material patches must survive the temporary presentation instance.
  copy.onBeforeCompile = source.onBeforeCompile;
  copy.customProgramCacheKey = source.customProgramCacheKey;
  if (source instanceof ShaderMaterial && copy instanceof ShaderMaterial) {
    copy.uniforms = { ...source.uniforms };
  }
  return copy;
}

function amountAt(state: SurfaceState, now: number): number {
  const t = Math.min(1, Math.max(0, (now - state.started) / TRANSITION_SECONDS));
  return state.from + (state.target - state.from) * (1 - (1 - t) ** 3);
}

/**
 * Presentation only: ray-test the moving body's screen coverage, then fade
 * individual foreground meshes. A transparent depth prepass admits only the
 * nearest faded surface at each pixel, so overlapping leaves blend once.
 * All clones belong to this controller; geometry/textures belong to the scene.
 */
export function createSurfaceOcclusion(
  surfaces: readonly OccludingSurface[],
  actor: Object3D,
  camera: OrthographicCamera,
) {
  actor.updateWorldMatrix(true, true);
  const actorSize = new Box3().setFromObject(actor).getSize(new Vector3());
  const height = actorSize.y;
  const shoulder = Math.min(actorSize.x, actorSize.z, height * 0.35) * 0.4;
  const states: SurfaceState[] = surfaces.map(surface => {
    surface.mesh.updateWorldMatrix(true, false);
    surface.mesh.geometry.computeBoundingBox();
    return {
      ...surface,
      bounds: surface.mesh.geometry.boundingBox!.clone().applyMatrix4(surface.mesh.matrixWorld),
      original: surface.mesh.material, order: surface.mesh.renderOrder,
      lastHit: -Infinity, amount: 0, from: 0, target: 0, started: 0,
    };
  });
  const raycaster = new Raycaster();
  const towardCamera = new Vector3();
  const right = new Vector3();
  const foot = new Vector3();
  const target = new Vector3();
  const intersection = new Vector3();
  let nextSample = -Infinity;

  function activate(state: SurfaceState): void {
    if (!state.colours) {
      state.colours = materials(state.original).map(source => {
        const colour = copyMaterial(source);
        colour.name = `${source.name}/occlusion-colour`;
        colour.transparent = true;
        colour.depthWrite = false;
        colour.depthFunc = EqualDepth;
        colour.forceSinglePass = true;
        if (colour instanceof ShaderMaterial) {
          colour.uniforms.surfaceOpacity = { value: 1 };
          // Rename the entry point rather than guessing where its final brace
          // falls among helper functions. Preserve discard and wind uniforms.
          if (!/void\s+main\s*\(\s*\)/.test(colour.fragmentShader)) {
            throw new Error("occluding shader has no main entry point");
          }
          colour.fragmentShader = "uniform float surfaceOpacity;\n" +
            colour.fragmentShader.replace(/void\s+main\s*\(\s*\)/, "void surfaceMain()") +
            "\nvoid main() { surfaceMain(); gl_FragColor.a *= surfaceOpacity; }\n";
        }
        return colour;
      });
      const depths = materials(state.original).map(source => {
        const depth = copyMaterial(source);
        depth.name = `${source.name}/occlusion-depth`;
        depth.transparent = true;
        depth.colorWrite = false;
        depth.depthWrite = true;
        depth.depthFunc = LessEqualDepth;
        depth.blending = NoBlending;
        depth.forceSinglePass = true;
        return depth;
      });
      state.depth = new Mesh(state.mesh.geometry, Array.isArray(state.original) ? depths : depths[0]!);
      state.depth.name = `${state.mesh.name}/occlusion-depth`;
      state.depth.renderOrder = 20;
      state.depth.matrixAutoUpdate = false;
      state.depth.castShadow = false;
    }
    state.mesh.material = Array.isArray(state.original) ? state.colours : state.colours[0]!;
    state.mesh.renderOrder = 21;
    state.mesh.parent!.add(state.depth!);
    state.depth!.matrix.copy(state.mesh.matrix);
    state.depth!.visible = state.mesh.visible;
  }

  function restore(state: SurfaceState): void {
    state.mesh.material = state.original;
    state.mesh.renderOrder = state.order;
    state.depth?.removeFromParent();
  }

  return {
    update(now: number): number {
      if (now >= nextSample) {
        nextSample = now + SAMPLE_SECONDS;
        actor.getWorldPosition(foot);
        camera.updateWorldMatrix(true, false);
        camera.getWorldDirection(towardCamera).negate();
        right.setFromMatrixColumn(camera.matrixWorld, 0);
        const selected = new Set<SurfaceState>();
        for (const fraction of [0.2, 0.5, 0.85]) {
          for (const lateral of [-shoulder, 0, shoulder]) {
            target.copy(foot).addScaledVector(right, lateral);
            target.y += height * fraction;
            const distance = camera.position.clone().sub(target).dot(towardCamera) - camera.near;
            if (distance <= 0) continue;
            raycaster.ray.origin.copy(target).addScaledVector(towardCamera, distance);
            raycaster.ray.direction.copy(towardCamera).negate();
            raycaster.near = 0;
            raycaster.far = distance - 0.025;
            for (const state of states) {
              if (selected.has(state) || !state.mesh.visible ||
                  !raycaster.ray.intersectBox(state.bounds, intersection) ||
                  intersection.distanceTo(raycaster.ray.origin) > raycaster.far) continue;
              if (raycaster.intersectObject(state.mesh, false).some(hit => !state.acceptsHit || state.acceptsHit(hit))) {
                selected.add(state);
                state.lastHit = now;
              }
            }
          }
        }
      }
      let count = 0;
      for (const state of states) {
        const targetAmount = now - state.lastHit <= RELEASE_SECONDS ? 1 : 0;
        if (targetAmount !== state.target) {
          state.from = amountAt(state, now);
          state.started = now;
          state.target = targetAmount;
        }
        state.amount = amountAt(state, now);
        if (state.amount <= 0) { restore(state); continue; }
        count += 1;
        activate(state);
        const scale = 1 - (1 - OCCLUDER_OPACITY) * state.amount;
        materials(state.original).forEach((source, index) => {
          const colour = state.colours![index]!;
          colour.opacity = source.opacity * scale;
          // Keep alpha-tested geometry and its shadow coverage while fading.
          colour.alphaTest = source.alphaTest * scale;
          if (colour instanceof ShaderMaterial) colour.uniforms.surfaceOpacity!.value = scale;
          if (source instanceof MeshStandardMaterial && colour instanceof MeshStandardMaterial) {
            colour.emissive.copy(source.emissive);
            colour.emissiveIntensity = source.emissiveIntensity;
          }
        });
      }
      return count;
    },
    snapshot() {
      return states.filter(state => state.amount > 0).map(state => ({
        name: `${state.mesh.parent?.name}/${state.mesh.name}`, id: state.mesh.id,
        opacity: 1 - (1 - OCCLUDER_OPACITY) * state.amount,
      }));
    },
    dispose(): void {
      for (const state of states) {
        restore(state);
        state.colours?.forEach(material => material.dispose());
        if (state.depth) materials(state.depth.material).forEach(material => material.dispose());
      }
    },
  };
}
