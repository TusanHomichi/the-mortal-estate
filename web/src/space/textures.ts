import {
  BufferAttribute,
  BufferGeometry,
  LinearFilter,
  LinearMipmapLinearFilter,
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
}

export async function decodeTextures(
  packet: VerifiedAssetPacket,
): Promise<Map<string, DecodedTexture>> {
  const decoded = new Map<string, DecodedTexture>();
  await Promise.all(
    [...packet.assets.entries()].map(async ([key, asset]) => {
      // ImageBitmap uploads ignore Texture.flipY in WebGL. Flip while decoding.
      const bitmap = await createImageBitmap(new Blob([asset.bytes], { type: "image/png" }), {
        imageOrientation: "flipY",
      });
      const texture = new Texture(bitmap);
      texture.name = key;
      texture.colorSpace = SRGBColorSpace;
      texture.wrapS = RepeatWrapping;
      texture.wrapT = RepeatWrapping;
      texture.magFilter = LinearFilter;
      texture.minFilter = LinearMipmapLinearFilter;
      texture.generateMipmaps = true;
      texture.needsUpdate = true;
      decoded.set(key, { texture, width: bitmap.width, height: bitmap.height });
    }),
  );
  return decoded;
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
