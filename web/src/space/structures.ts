import { Box3, Group, Mesh } from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import type { StructurePlacement, VerifiedAssetPacket } from "../feelTypes";
import { disposeFigureSources } from "./figureRig";

export function parseStructures(value: unknown, extents: { i: number; j: number }): StructurePlacement[] {
  if (!Array.isArray(value)) throw new Error("candidate structures must be a list");
  return value.map((row: unknown) => {
    if (typeof row !== "object" || row === null || Array.isArray(row)) throw new Error("invalid candidate structure");
    const v = row as Record<string, unknown>;
    const keys = ["file", "sha256", "cell_anchor", "yaw", "footprint"];
    if (Object.keys(v).length !== keys.length || Object.keys(v).some((k) => !keys.includes(k)) ||
        typeof v.file !== "string" || !/^[A-Za-z0-9_-]+\.glb$/.test(v.file) ||
        typeof v.sha256 !== "string" || !/^[0-9a-f]{64}$/.test(v.sha256) ||
        !Array.isArray(v.cell_anchor) || v.cell_anchor.length !== 2 ||
        !v.cell_anchor.every((n) => typeof n === "number" && Number.isFinite(n)) ||
        typeof v.yaw !== "number" || ![0, 90, 180, 270].includes(v.yaw)) {
      throw new Error("candidate structure has invalid files, placement, or fields");
    }
    const f = v.footprint as Record<string, unknown> | null;
    if (typeof f !== "object" || f === null || Array.isArray(f) ||
        Object.keys(f).length !== 4 || !["i0", "j0", "i1", "j1"].every((k) => Number.isInteger(f[k]))) {
      throw new Error("candidate structure footprint is invalid");
    }
    const footprint = f as StructurePlacement["footprint"];
    if (footprint.i0 < 0 || footprint.j0 < 0 || footprint.i1 >= extents.i || footprint.j1 >= extents.j ||
        footprint.i0 > footprint.i1 || footprint.j0 > footprint.j1 ||
        v.cell_anchor[0] < footprint.i0 - 0.5 || v.cell_anchor[0] > footprint.i1 + 0.5 ||
        v.cell_anchor[1] < footprint.j0 - 0.5 || v.cell_anchor[1] > footprint.j1 + 0.5) {
      throw new Error("candidate structure footprint or anchor is outside its space");
    }
    return { file: v.file, sha256: v.sha256, cell_anchor: [...v.cell_anchor] as [number, number], yaw: v.yaw, footprint: { ...footprint } };
  });
}

/** A GLB must carry every buffer and image inside its verified bytes. */
export function assertEmbeddedStructure(bytes: ArrayBuffer): void {
  if (bytes.byteLength < 20) throw new Error("structure is not a complete GLB");
  const view = new DataView(bytes);
  const length = view.getUint32(12, true);
  if (view.getUint32(0, true) !== 0x46546c67 || view.getUint32(4, true) !== 2 ||
      view.getUint32(8, true) !== bytes.byteLength || view.getUint32(16, true) !== 0x4e4f534a ||
      length > bytes.byteLength - 20) throw new Error("structure is not a valid GLB");
  const doc = JSON.parse(new TextDecoder().decode(new Uint8Array(bytes, 20, length)));
  for (const row of [...(doc.buffers ?? []), ...(doc.images ?? [])]) {
    if (Object.hasOwn(row, "uri")) throw new Error("structure references an external buffer or image");
  }
  if ((doc.animations?.length ?? 0) || (doc.skins?.length ?? 0)) throw new Error("structures must be static meshes");
}

/** Decoded once; instances share immutable source geometry and materials. */
export async function decodeStructures(packet: VerifiedAssetPacket): Promise<Map<string, Group>> {
  const decoded = new Map<string, Group>();
  try {
    for (const [space, plan] of Object.entries(packet.manifest.spaces)) {
      for (const [index, placement] of plan.structures.entries()) {
        const key = `structures/${space}/${index}`;
        const bytes = packet.assets.get(key)?.bytes;
        if (!bytes) throw new Error(`${key} was not verified`);
        assertEmbeddedStructure(bytes);
        const gltf = await new GLTFLoader().parseAsync(bytes, "");
        decoded.set(key, gltf.scene);
        let meshes = 0;
        gltf.scene.traverse((o) => {
          if (!(o instanceof Mesh)) return;
          meshes += 1;
          o.userData.sharedStructure = true;
          o.castShadow = true;
          o.receiveShadow = true;
        });
        if (!meshes) throw new Error(`${key} carries no meshes`);
        const box = new Box3().setFromObject(gltf.scene);
        if (![...box.min.toArray(), ...box.max.toArray()].every(Number.isFinite)) throw new Error(`${key} has invalid bounds`);
        // An overhanging roof may project outside occupancy; the footprint is
        // an authored ground boundary, never derived from the visible bounds.
        void placement;
      }
    }
    return decoded;
  } catch (error) {
    disposeFigureSources([...decoded.values()]);
    throw error;
  }
}

export function addStructures(parent: Group, name: string, placements: readonly StructurePlacement[], decoded: ReadonlyMap<string, Group>): void {
  placements.forEach((placement, index) => {
    const source = decoded.get(`structures/${name}/${index}`);
    if (!source) throw new Error(`structure ${name}/${index} was not decoded`);
    const root = new Group();
    root.name = `Structure_${index}`;
    root.add(source.clone(true));
    root.position.set(placement.cell_anchor[0], 0, placement.cell_anchor[1]);
    root.rotation.y = placement.yaw * Math.PI / 180;
    parent.add(root);
  });
}
