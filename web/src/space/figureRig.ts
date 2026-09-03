import {
  AnimationClip,
  AnimationMixer,
  Group,
  LoadingManager,
  Material,
  Mesh,
  MeshStandardMaterial,
  Object3D,
  PropertyBinding,
  SkinnedMesh,
  Texture,
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

/**
 * The palette patch only exists for three's physical material. A glTF may
 * carry other kinds — KHR_materials_unlit decodes to a basic material — and
 * such a figure would render unpainted; it is refused at decode instead.
 */
export function assertPaintableMaterials(root: Object3D, what: string): void {
  root.traverse((object: Object3D) => {
    if (!(object instanceof Mesh)) return;
    const materials = Array.isArray(object.material) ? object.material : [object.material];
    for (const material of materials) {
      if (!(material instanceof MeshStandardMaterial)) {
        throw new Error(`${what} carries a ${material.type} on ${object.name || "a mesh"}; the figure palette patches only standard materials`);
      }
    }
  });
}

/**
 * A clip binds by node name. Three's mixer treats a track it cannot bind as
 * a warning and plays the rest, so a plain mesh, a part on another skeleton,
 * or a library from a different rig would decode and simply not move. Every
 * track must find its node on the rig and on every part; otherwise the figure
 * is refused, naming the first bone it cannot find.
 */
export function assertClipBinds(clip: AnimationClip, root: Object3D, what: string): void {
  // The names that count are the bones of the skinned meshes' skeletons —
  // a plain hierarchy carrying the right names would bind and move nothing.
  const bones = new Set<string>();
  root.traverse((object: Object3D) => {
    if (object instanceof SkinnedMesh) for (const bone of object.skeleton.bones) bones.add(bone.name);
  });
  if (bones.size === 0) throw new Error(`${what} has no skinned mesh; ${clip.name} has nothing to move`);
  for (const track of clip.tracks) {
    const { nodeName } = PropertyBinding.parseTrackName(track.name);
    if (nodeName === undefined || nodeName === "" || !bones.has(nodeName)) {
      throw new Error(`${what} cannot play ${clip.name}: no skeleton bone named ${String(nodeName)}`);
    }
  }
}

/** The bone names of every skinned mesh under a root, as one set. */
function skeletonBoneNames(root: Object3D): Set<string> {
  const bones = new Set<string>();
  root.traverse((object: Object3D) => {
    if (object instanceof SkinnedMesh) for (const bone of object.skeleton.bones) bones.add(bone.name);
  });
  return bones;
}

/**
 * A part is mixed on its own skeleton, so it stays aligned with the rig only
 * if it is the same skeleton: the same bones, no more and no fewer. A part
 * built for another rig would bind the clip and deform apart from the body.
 */
export function assertSameSkeleton(rig: Object3D, part: Object3D, what: string): void {
  const rigBones = skeletonBoneNames(rig);
  const partBones = skeletonBoneNames(part);
  const missing = [...rigBones].filter((bone) => !partBones.has(bone));
  const extra = [...partBones].filter((bone) => !rigBones.has(bone));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `${what} is not on the rig's skeleton: ` +
        `${missing.length} bone(s) missing${missing.length ? ` (${missing.slice(0, 3).join(", ")})` : ""}, ` +
        `${extra.length} extra${extra.length ? ` (${extra.slice(0, 3).join(", ")})` : ""}`,
    );
  }
}

/** Skinned meshes own a skeleton whose bone texture lives on the GPU; release each one. */
function disposeSkeletons(root: Object3D): void {
  root.traverse((object: Object3D) => {
    if (object instanceof SkinnedMesh) object.skeleton.dispose();
  });
}

function figureFiles(figure: FigureRow): string[] {
  return [figure.rig, ...figure.sidecars, figure.clips, ...figure.parts, ...figure.parts.flatMap((part) => part.sidecars)]
    .map((file) => file.file);
}

/** Decodes every figure the packet carries, from verified bytes and nothing else. */
export async function decodeFigures(packet: VerifiedAssetPacket): Promise<Map<string, DecodedFigure>> {
  const decoded = new Map<string, DecodedFigure>();
  // A refusal anywhere refuses the packet; whatever was parsed before it —
  // earlier figures, or this figure's rig and parts — is released first.
  const parsed: Object3D[] = [];
  try {
    await decodeFiguresInto(packet, decoded, parsed);
  } catch (error) {
    disposeFigureSources(parsed);
    throw error;
  }
  return decoded;
}

async function decodeFiguresInto(
  packet: VerifiedAssetPacket,
  decoded: Map<string, DecodedFigure>,
  parsed: Object3D[],
): Promise<void> {
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
      parsed.push(rig.scene);
      assertPaintableMaterials(rig.scene, `figure ${name} rig`);
      const parts: Group[] = [];
      for (const part of figure.parts) {
        const scene = (await parseGltf(loader, bytesOf(part.file), `figure ${name} part ${part.file}`)).scene;
        parsed.push(scene);
        assertPaintableMaterials(scene, `figure ${name} part ${part.file}`);
        parts.push(scene);
      }
      const clips = (await parseGltf(loader, bytesOf(figure.clips.file), `figure ${name} clips`)).animations;
      const idle = clips.find((clip) => clip.name === figure.idle);
      if (idle === undefined) throw new Error(`figure ${name} has no clip named ${figure.idle}`);
      assertClipBinds(idle, rig.scene, `figure ${name} rig`);
      parts.forEach((part, index) => {
        const what = `figure ${name} part ${figure.parts[index]!.file}`;
        assertSameSkeleton(rig.scene, part, what);
        assertClipBinds(idle, part, what);
      });
      decoded.set(name, { name, rig: rig.scene, parts, clips, palette: figure.palette, rim: figure.rim, idle: figure.idle });
    } finally {
      for (const url of table.values()) URL.revokeObjectURL(url);
    }
  }
}

/**
 * Releases what decoding created — the source rigs' and parts' geometry, the
 * materials the instances cloned from, and their textures. Instances share the
 * geometry and dispose only their own materials, so this runs once, when the
 * scene that decoded the figures stops.
 */
export function disposeDecodedFigures(figures: ReadonlyMap<string, DecodedFigure>): void {
  disposeFigureSources([...figures.values()].flatMap((figure) => [figure.rig, ...figure.parts]));
}

/** Releases parsed glTF scenes: skeletons, geometry, materials, and their textures. */
export function disposeFigureSources(sources: readonly Object3D[]): void {
  const materials = new Set<Material>();
  for (const source of sources) {
    disposeSkeletons(source);
    source.traverse((object: Object3D) => {
      if (!(object instanceof Mesh)) return;
      object.geometry.dispose();
      const owned = Array.isArray(object.material) ? object.material : [object.material];
      for (const material of owned) materials.add(material);
    });
  }
  for (const material of materials) {
    for (const value of Object.values(material)) {
      if (value instanceof Texture) value.dispose();
    }
    material.dispose();
  }
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
        if (!(material instanceof MeshStandardMaterial)) {
          throw new Error(`figure ${figure.name} carries a ${material.type}; the palette patches only standard materials`);
        }
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
      disposeSkeletons(root);
      for (const material of ownedMaterials) material.dispose();
    },
  };
  instance.place(cell.i, cell.j);
  instance.setFacing(facing);
  return instance;
}
