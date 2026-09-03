import {
  AnimationClip,
  AnimationMixer,
  Group,
  LoadingManager,
  Material,
  Mesh,
  MeshStandardMaterial,
  Object3D,
  Vector3,
} from "three";
import { GLTFLoader, type GLTF } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneWithSkeleton } from "three/examples/jsm/utils/SkeletonUtils.js";
import type { Cell } from "../walk/layoutPassability";
import type { FigureRow, VerifiedAssetPacket } from "../feelTypes";

/**
 * The live figure (owner ruling, 2026-09-03): a rigged glTF, its outfit parts
 * on the same skeleton, a clip library, and a material that reproduces the
 * treated cards' grammar — the figure's own palette as a nearest-colour lookup
 * after lighting, with a rim darkening. Decoded once per packet from verified
 * bytes only; instanced per space with a cloned skeleton.
 */
export interface DecodedFigure {
  name: string;
  rig: Group;
  parts: Group[];
  clips: AnimationClip[];
  palette: [number, number, number][];
  rim: number;
  idle: string;
}

/** What the walk presenter holds: one object to place and face, ticked by the scene. */
export interface FigureInstance {
  readonly name: string;
  readonly root: Group;
  readonly clip: string;
  facing: 1 | -1;
  place(i: number, j: number): void;
  setFacing(direction: 1 | -1): void;
  update(deltaSeconds: number): void;
  dispose(): void;
}

/**
 * A glTF names its buffers and textures by relative URI. Every name the loader
 * asks for resolves here against the figure's verified files, and nothing else:
 * an unlisted name refuses the figure rather than reaching the network.
 */
export function resolveFigureUrl(url: string, table: ReadonlyMap<string, string>): string {
  const withoutQuery = url.split(/[?#]/, 1)[0] ?? url;
  let name = withoutQuery.slice(withoutQuery.lastIndexOf("/") + 1);
  try {
    name = decodeURIComponent(name);
  } catch {
    // an undecodable name is simply not in the table
  }
  const resolved = table.get(name);
  if (resolved === undefined) throw new Error(`a figure names an unlisted file: ${name}`);
  return resolved;
}

const PALETTE_ANCHOR = "#include <opaque_fragment>";

/**
 * Patches a physical material so its lit colour snaps, in gamma space, to the
 * nearest of the figure's palette after a rim darkening. The anchor is three's
 * own; if an upgrade moves it the patch refuses loudly rather than shipping an
 * unpainted figure.
 */
export function applyFigurePalette(
  material: MeshStandardMaterial,
  palette: readonly (readonly [number, number, number])[],
  rim: number,
): void {
  const count = palette.length;
  material.onBeforeCompile = (shader) => {
    if (!shader.fragmentShader.includes(PALETTE_ANCHOR)) {
      throw new Error("three's fragment shader no longer carries the anchor the figure palette patches");
    }
    shader.uniforms["figurePalette"] = {
      value: palette.map((colour) => new Vector3(colour[0] / 255, colour[1] / 255, colour[2] / 255)),
    };
    shader.fragmentShader = `uniform vec3 figurePalette[${count}];\n` + shader.fragmentShader.replace(
      PALETTE_ANCHOR,
      `
      {
        vec3 g = pow(max(outgoingLight, vec3(0.0)), vec3(1.0 / 2.2));
        float ndv = abs(dot(normalize(normal), vec3(0.0, 0.0, 1.0)));
        g *= 1.0 - ${rim.toFixed(4)} * (1.0 - smoothstep(0.0, 0.4, ndv));
        vec3 w = vec3(0.6, 1.0, 0.4);
        float best = 1e9;
        vec3 pick = g;
        for (int i = 0; i < ${count}; i++) {
          vec3 d = (figurePalette[i] - g) * w;
          float dist = dot(d, d);
          if (dist < best) { best = dist; pick = figurePalette[i]; }
        }
        outgoingLight = pow(clamp(pick, 0.0, 1.0), vec3(2.2));
      }
      ${PALETTE_ANCHOR}`,
    );
  };
  material.customProgramCacheKey = () => `figure-palette-${count}-${rim}`;
  material.needsUpdate = true;
}

function parseGltf(loader: GLTFLoader, bytes: ArrayBuffer, what: string): Promise<GLTF> {
  return new Promise((resolve, reject) => {
    loader.parse(bytes, "", resolve, (error) => reject(new Error(`${what} could not be decoded: ${String(error)}`)));
  });
}

function figureFiles(figure: FigureRow): string[] {
  return [figure.rig, ...figure.sidecars, figure.clips, ...figure.parts, ...figure.parts.flatMap((part) => part.sidecars)]
    .map((file) => file.file);
}

/** Decodes every figure the packet carries, from verified bytes and nothing else. */
export async function decodeFigures(packet: VerifiedAssetPacket): Promise<Map<string, DecodedFigure>> {
  const decoded = new Map<string, DecodedFigure>();
  for (const [name, figure] of Object.entries(packet.manifest.figures)) {
    const table = new Map<string, string>();
    const bytesOf = (file: string): ArrayBuffer => {
      const asset = packet.assets.get(`figures/${name}/${file}`);
      if (asset === undefined) throw new Error(`figure ${name} file ${file} was not verified`);
      return asset.bytes;
    };
    for (const file of figureFiles(figure)) table.set(file, URL.createObjectURL(new Blob([bytesOf(file)])));
    try {
      const manager = new LoadingManager();
      manager.setURLModifier((url) => resolveFigureUrl(url, table));
      const loader = new GLTFLoader(manager);
      const rig = await parseGltf(loader, bytesOf(figure.rig.file), `figure ${name} rig`);
      const parts: Group[] = [];
      for (const part of figure.parts) parts.push((await parseGltf(loader, bytesOf(part.file), `figure ${name} part ${part.file}`)).scene);
      const clips = (await parseGltf(loader, bytesOf(figure.clips.file), `figure ${name} clips`)).animations;
      if (!clips.some((clip) => clip.name === figure.idle)) {
        throw new Error(`figure ${name} has no clip named ${figure.idle}`);
      }
      decoded.set(name, { name, rig: rig.scene, parts, clips, palette: figure.palette, rim: figure.rim, idle: figure.idle });
    } finally {
      for (const url of table.values()) URL.revokeObjectURL(url);
    }
  }
  return decoded;
}

// Facing: the rig's front is +z; a two-way facing turns it along ±x, the axis
// the card used to mirror across (decided 2026-09-03; eight-way is open).
const FACING_YAW: Record<1 | -1, number> = { 1: Math.PI / 2, [-1]: -Math.PI / 2 };

/** Instances a decoded figure at a cell: cloned skeletons, patched materials, the idle clip playing. */
export function createFigureInstance(figure: DecodedFigure, cell: Cell, facing: 1 | -1): FigureInstance {
  const root = new Group();
  root.name = `Figure_${figure.name}`;
  const mixers: AnimationMixer[] = [];
  const ownedMaterials: Material[] = [];
  const idle = figure.clips.find((clip) => clip.name === figure.idle);
  if (idle === undefined) throw new Error(`figure ${figure.name} has no clip named ${figure.idle}`);
  for (const source of [figure.rig, ...figure.parts]) {
    const part = cloneWithSkeleton(source) as Group;
    part.traverse((object: Object3D) => {
      if (!(object instanceof Mesh)) return;
      object.castShadow = true;
      object.receiveShadow = true;
      const materials = Array.isArray(object.material) ? object.material : [object.material];
      const patched = materials.map((material) => {
        if (!(material instanceof MeshStandardMaterial)) return material;
        const own = material.clone();
        applyFigurePalette(own, figure.palette, figure.rim);
        ownedMaterials.push(own);
        return own;
      });
      object.material = Array.isArray(object.material) ? patched : patched[0]!;
    });
    const mixer = new AnimationMixer(part);
    mixer.clipAction(idle).play();
    mixers.push(mixer);
    root.add(part);
  }
  const instance: FigureInstance = {
    name: figure.name,
    root,
    clip: idle.name,
    facing,
    place(i, j) {
      root.position.set(i, 0, j);
    },
    setFacing(direction) {
      instance.facing = direction;
      root.rotation.y = FACING_YAW[direction];
    },
    update(deltaSeconds) {
      if (deltaSeconds <= 0) return;
      for (const mixer of mixers) mixer.update(deltaSeconds);
    },
    dispose() {
      root.removeFromParent();
      for (const material of ownedMaterials) material.dispose();
    },
  };
  instance.place(cell.i, cell.j);
  instance.setFacing(facing);
  return instance;
}
