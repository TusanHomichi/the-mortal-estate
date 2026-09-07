import type { Intersection } from "three";
import type { DecodedTexture } from "./textures";

/** Decode-time pixels are top-down; transformUv includes the texture's flip. */
export function textureHitTest(source: DecodedTexture): (hit: Intersection) => boolean {
  if (!source.pixels) throw new Error("occluding card was decoded without readable alpha");
  const pixels = source.pixels;
  return hit => {
    if (!hit.uv) return false;
    source.texture.updateMatrix();
    const uv = source.texture.transformUv(hit.uv.clone());
    const x = Math.min(source.width - 1, Math.max(0, Math.floor(uv.x * source.width)));
    const y = Math.min(source.height - 1, Math.max(0, Math.floor(uv.y * source.height)));
    return pixels[(y * source.width + x) * 4 + 3]! / 255 >= 0.12;
  };
}
