import { afterEach, describe, expect, it, vi } from "vitest";
import type { VerifiedAssetPacket } from "../src/feelTypes";
import { assertNormalSheetsMatch, decodeTextures, isFigureKey, isNormalSheetKey } from "../src/space/textures";

describe("normal sheets at decode time", () => {
  it("names a normal sheet by its key suffix", () => {
    expect(isNormalSheetKey("props/caretaker/normal")).toBe(true);
    expect(isNormalSheetKey("props/caretaker")).toBe(false);
  });

  it("accepts a normal sheet the size of its colour sheet", () => {
    const textures = new Map([
      ["props/caretaker", { width: 1254, height: 1254 }],
      ["props/caretaker/normal", { width: 1254, height: 1254 }],
      ["terrain/grass", { width: 512, height: 512 }],
    ]);
    expect(() => assertNormalSheetsMatch(textures)).not.toThrow();
  });

  it("refuses a mismatched normal sheet before any space is built", () => {
    const textures = new Map([
      ["props/kit_bed_twin1", { width: 760, height: 1428 }],
      ["props/kit_bed_twin1/normal", { width: 760, height: 1400 }],
    ]);
    expect(() => assertNormalSheetsMatch(textures)).toThrow(/760x1400 but its colour sheet is 760x1428/);
  });

  it("refuses a normal sheet with no colour sheet", () => {
    const textures = new Map([["props/ghost/normal", { width: 8, height: 8 }]]);
    expect(() => assertNormalSheetsMatch(textures)).toThrow(/has no colour sheet props\/ghost/);
  });
});

describe("decoding a sheet", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("decodes every sheet straight, never through a premultiplied round trip", async () => {
    const requests: ImageBitmapOptions[] = [];
    vi.stubGlobal(
      "createImageBitmap",
      vi.fn(async (_blob: Blob, options?: ImageBitmapOptions) => {
        requests.push(options ?? {});
        return { width: 4, height: 4, close: () => undefined } as unknown as ImageBitmap;
      }),
    );
    const packet = {
      manifest: { schema_version: 3, assets: {}, start: { space: "s", cell: [0, 0] }, spaces: {} },
      assets: new Map([
        ["props/caretaker", { bytes: new ArrayBuffer(0), file: "prop-caretaker.png" }],
        ["props/caretaker/normal", { bytes: new ArrayBuffer(0), file: "prop-caretaker-normal.png" }],
      ]),
    } as unknown as VerifiedAssetPacket;
    packet.assets.set("figures/caretaker/figure.gltf", { bytes: new ArrayBuffer(0), file: "figure.gltf" });
    const decoded = await decodeTextures(packet);
    // The figure's files are not sheets and are never decoded as images.
    expect(isFigureKey("figures/caretaker/figure.gltf")).toBe(true);
    expect(decoded.size).toBe(2);
    expect(requests).toHaveLength(2);
    for (const options of requests) {
      // The default round trip zeroes the colour under alpha 0; a normal
      // sheet's surround would come back black and light the silhouette ring.
      expect(options.premultiplyAlpha).toBe("none");
      expect(options.imageOrientation).toBe("flipY");
    }
  });
});
