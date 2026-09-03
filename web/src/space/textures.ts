import {
  BufferAttribute,
  BufferGeometry,
  LinearFilter,
  LinearMipmapLinearFilter,
  NoColorSpace,
  RepeatWrapping,
  SRGBColorSpace,
  Texture,
} from "three";
import type { VerifiedAssetPacket } from "../feelTypes";
import type { GeometryData } from "../wallGeometry";

export interface DecodedTexture {
  texture: Texture;
  width: number;
  height: number;
  pixels: Uint8ClampedArray | null;
}

export async function decodeTextures(
  packet: VerifiedAssetPacket,
): Promise<Map<string, DecodedTexture>> {
  const decoded = new Map<string, DecodedTexture>();
  const windTextureKeys = new Set(
    Object.values(packet.manifest.spaces).flatMap((space) =>
      space.props
        .filter((prop) => prop.sway)
        .map((prop) => `props/${prop.kind}`)
    ),
  );
  if (packet.assets.has("props/grass_clump")) windTextureKeys.add("props/grass_clump");
  await Promise.all(
    [...packet.assets.entries()].map(async ([key, asset]) => {
      const blob = new Blob([asset.bytes], { type: "image/png" });
      let pixels: Uint8ClampedArray | null = null;
      if (windTextureKeys.has(key)) {
        const readableBitmap = await createImageBitmap(blob);
        const canvas = document.createElement("canvas");
        canvas.width = readableBitmap.width;
        canvas.height = readableBitmap.height;
        const canvasContext = canvas.getContext("2d", { willReadFrequently: true });
        if (canvasContext === null) throw new Error(`decoded texture ${key} has no 2D context`);
        canvasContext.drawImage(readableBitmap, 0, 0);
        pixels = canvasContext.getImageData(
          0,
          0,
          readableBitmap.width,
          readableBitmap.height,
        ).data;
        readableBitmap.close();
      }
      // ImageBitmap uploads ignore Texture.flipY in WebGL. Flip while decoding.
      // Decode straight, never premultiplied: the browser's default round trip
      // (premultiply on decode, un-premultiply on upload) zeroes the colour of
      // every fully transparent pixel, so a normal sheet's flat surround becomes
      // black, and filtering pulls that black into the silhouette ring as a
      // backward normal — a bright outline around every card at every edge.
      const bitmap = await createImageBitmap(blob, {
        imageOrientation: "flipY",
        premultiplyAlpha: "none",
      });
      const texture = new Texture(bitmap);
      texture.name = key;
      // A normal sheet is data, not colour; it must not be sRGB-decoded.
      texture.colorSpace = isNormalSheetKey(key) ? NoColorSpace : SRGBColorSpace;
      texture.wrapS = RepeatWrapping;
      texture.wrapT = RepeatWrapping;
      texture.magFilter = LinearFilter;
      texture.minFilter = LinearMipmapLinearFilter;
      texture.generateMipmaps = true;
      texture.needsUpdate = true;
      decoded.set(key, { texture, width: bitmap.width, height: bitmap.height, pixels });
    }),
  );
  assertNormalSheetsMatch(decoded);
  return decoded;
}

/**
 * Every normal sheet must be the pixel size of the colour sheet it belongs
 * to. Checked once, here, before any space is built: a mismatch refuses the
 * packet up front instead of surfacing when a portal swaps to the space that
 * uses it, after the previous space has already been disposed.
 */
export function assertNormalSheetsMatch(
  textures: ReadonlyMap<string, { width: number; height: number }>,
): void {
  for (const [key, normal] of textures) {
    if (!isNormalSheetKey(key)) continue;
    const colourKey = key.slice(0, -"/normal".length);
    const colour = textures.get(colourKey);
    if (colour === undefined) {
      throw new Error(`the normal sheet ${key} has no colour sheet ${colourKey}`);
    }
    if (colour.width !== normal.width || colour.height !== normal.height) {
      throw new Error(
        `the normal sheet ${key} is ${normal.width}x${normal.height} but its colour sheet is ${colour.width}x${colour.height}`,
      );
    }
  }
}

export function isNormalSheetKey(key: string): boolean {
  return key.endsWith("/normal");
}

export function requiredTexture(
  textures: Map<string, DecodedTexture>,
  key: string,
): DecodedTexture {
  const texture = textures.get(key);
  if (texture === undefined) throw new Error(`verified texture ${key} was not decoded`);
  return texture;
}

export function configureTexture(texture: Texture, anisotropy: number): void {
  texture.anisotropy = anisotropy;
  texture.needsUpdate = true;
}

export function geometryFromData(data: GeometryData): BufferGeometry {
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new BufferAttribute(new Float32Array(data.positions), 3));
  geometry.setAttribute("uv", new BufferAttribute(new Float32Array(data.uvs), 2));
  geometry.setIndex(data.indices);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}
