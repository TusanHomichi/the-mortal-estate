import { ShaderChunk, type MeshStandardMaterial } from "three";

/**
 * Wrapped-diffuse width for every card material. Zero is plain Lambert; one
 * lets a card take direct light from the whole hemisphere. A half keeps a face
 * lit from the side or behind readable without silvering dark iron under the
 * night key — the 2026-09-03 capture pair against 1.0 chose it. The same width
 * applies to the key light and to every practical.
 */
export const CARD_DIFFUSE_WRAP = 0.5;

const wrap = CARD_DIFFUSE_WRAP.toFixed(2);

/** GLSL shared by the physical-material patch and the wind card shader. */
export const cardWrappedDiffuseGlsl = /* glsl */ `
  float cardWrappedDiffuse(vec3 normal, vec3 lightDirection) {
    return clamp((dot(normal, lightDirection) + ${wrap}) / (1.0 + ${wrap}), 0.0, 1.0);
  }
`;

const PHYSICAL_LIGHTING_INCLUDE = "#include <lights_physical_pars_fragment>";
const LAMBERT_ANCHOR =
  "float dotNL = saturate( dot( geometryNormal, directLight.direction ) );";
const WRAPPED_LAMBERT =
  "float dotNL = cardWrappedDiffuse( geometryNormal, directLight.direction );";

/**
 * Rewrites a physical fragment shader so its direct lighting uses the card
 * wrap. The anchor is three's own Lambert line; if a three upgrade moves or
 * renames it the patch refuses loudly rather than silently shipping unwrapped
 * cards.
 */
export function wrapCardDiffuse(fragmentShader: string): string {
  if (!fragmentShader.includes(PHYSICAL_LIGHTING_INCLUDE)) {
    throw new Error("card lighting expects a physical fragment shader to patch");
  }
  const chunk = ShaderChunk.lights_physical_pars_fragment;
  if (chunk.split(LAMBERT_ANCHOR).length !== 2) {
    throw new Error("three's physical lighting chunk no longer carries the Lambert anchor the card wrap patches");
  }
  return fragmentShader.replace(
    PHYSICAL_LIGHTING_INCLUDE,
    `${cardWrappedDiffuseGlsl}\n${chunk.replace(LAMBERT_ANCHOR, WRAPPED_LAMBERT)}`,
  );
}

/** Gives a card's standard material the wrap, on its own program cache entry. */
export function applyCardLighting(material: MeshStandardMaterial): void {
  material.onBeforeCompile = (shader) => {
    shader.fragmentShader = wrapCardDiffuse(shader.fragmentShader);
  };
  material.customProgramCacheKey = () => `card-wrapped-diffuse-${wrap}`;
}
