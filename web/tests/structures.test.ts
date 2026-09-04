import { describe, expect, it } from "vitest";
import { assertEmbeddedStructure, parseStructures } from "../src/space/structures";

const row = { file: "cottage.glb", sha256: "a".repeat(64), cell_anchor: [3, 3], yaw: 270,
  footprint: { i0: 2, j0: 2, i1: 4, j1: 4 } };
function glb(document: unknown): ArrayBuffer {
  const text = JSON.stringify(document);
  const json = new TextEncoder().encode(text + " ".repeat((4 - text.length % 4) % 4));
  const bytes = new ArrayBuffer(20 + json.length);
  const view = new DataView(bytes);
  [0x46546c67, 2, bytes.byteLength, json.length, 0x4e4f534a].forEach((v, i) => view.setUint32(i * 4, v, true));
  new Uint8Array(bytes, 20).set(json);
  return bytes;
}
describe("static structure packet boundary", () => {
  it("requires a digest-bound GLB and a bounded authored footprint", () => {
    expect(parseStructures([row], { i: 8, j: 8 })).toEqual([row]);
    for (const bad of [{ ...row, file: "../cottage.glb" }, { ...row, sha256: "" },
      { ...row, footprint: { i0: 0, j0: 0, i1: 9, j1: 4 } }, { ...row, yaw: 45 },
      { ...row, scale: 2 }]) expect(() => parseStructures([bad], { i: 8, j: 8 })).toThrow();
  });
  it("refuses unverified external references and animated structures", () => {
    expect(() => assertEmbeddedStructure(glb({ buffers: [{ byteLength: 0 }], images: [] }))).not.toThrow();
    for (const doc of [{ buffers: [{ uri: "remote.bin" }] }, { images: [{ uri: "data:image/png;base64,x" }] },
      { skins: [{}] }, { animations: [{}] }]) expect(() => assertEmbeddedStructure(glb(doc))).toThrow();
    expect(() => assertEmbeddedStructure(new ArrayBuffer(12))).toThrow();
  });
});
