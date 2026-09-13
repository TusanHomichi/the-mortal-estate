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
