import { MeshStandardMaterial, ShaderChunk, ShaderLib } from "three";
import { describe, expect, it } from "vitest";
import {
  applyCardLighting,
  CARD_DIFFUSE_WRAP,
  cardWrappedDiffuseGlsl,
  wrapCardDiffuse,
} from "../src/space/cardLighting";
import { windFragmentShader, windVertexShader } from "../src/shaders";

describe("card lighting", () => {
  it("wraps the installed three's Lambert term exactly once", () => {
    const anchor = "float dotNL = saturate( dot( geometryNormal, directLight.direction ) );";
    expect(ShaderChunk.lights_physical_pars_fragment.split(anchor)).toHaveLength(2);

    const patched = wrapCardDiffuse(ShaderLib.physical.fragmentShader);
    expect(patched).not.toContain("#include <lights_physical_pars_fragment>");
    expect(patched).not.toContain(anchor);
    expect(patched).toContain("float dotNL = cardWrappedDiffuse( geometryNormal, directLight.direction );");
    expect(patched.indexOf("float cardWrappedDiffuse(")).toBeLessThan(
      patched.indexOf("void RE_Direct_Physical("),
    );
  });

  it("refuses to patch a shader that has no physical lighting to wrap", () => {
    expect(() => wrapCardDiffuse(ShaderLib.basic.fragmentShader)).toThrow(/physical fragment shader/);
  });

  it("bakes the ruled wrap width into the shared GLSL", () => {
    expect(CARD_DIFFUSE_WRAP).toBe(0.5);
    expect(cardWrappedDiffuseGlsl).toContain("+ 0.50) / (1.0 + 0.50)");
  });

  it("gives a standard material its own program cache entry", () => {
    const material = new MeshStandardMaterial();
    applyCardLighting(material);
    expect(material.customProgramCacheKey()).toContain("card-wrapped-diffuse");
    const shader = { fragmentShader: ShaderLib.physical.fragmentShader, vertexShader: "" };
    material.onBeforeCompile(shader as never, null as never);
    expect(shader.fragmentShader).toContain("cardWrappedDiffuse( geometryNormal");
  });

  it("lights wind cards through the same wrap and an optional normal sheet", () => {
    expect(windFragmentShader).toContain("float cardWrappedDiffuse(");
    expect(windFragmentShader).toContain("uniform vec3 keyDirection;");
    expect(windFragmentShader).toContain("#ifdef CARD_NORMAL_MAP");
    expect(windFragmentShader).toContain("texture2D(normalTexture, sampleUv)");
    expect(windFragmentShader).not.toContain("keyColour * 0.48");
    expect(windVertexShader).toContain("vTangent");
  });
});
