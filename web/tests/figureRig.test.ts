import { describe, expect, it } from "vitest";
import { MeshStandardMaterial } from "three";
import { applyFigurePalette, resolveFigureUrl } from "../src/space/figureRig";

describe("figure files resolve only against verified bytes", () => {
  const table = new Map([["figure.bin", "blob:figure-bin"], ["T_Base.png", "blob:base"]]);

  it("maps a glTF's relative name, with or without a path or query, to its verified blob", () => {
    expect(resolveFigureUrl("figure.bin", table)).toBe("blob:figure-bin");
    expect(resolveFigureUrl("/feel-assets/figure.bin?x=1", table)).toBe("blob:figure-bin");
    expect(resolveFigureUrl("T_Base.png", table)).toBe("blob:base");
  });

  it("refuses a name the figure does not carry", () => {
    expect(() => resolveFigureUrl("T_Other.png", table)).toThrow(/unlisted file: T_Other.png/);
    expect(() => resolveFigureUrl("https://example.invalid/figure.bin", table)).not.toThrow();
    expect(() => resolveFigureUrl("https://example.invalid/evil.bin", table)).toThrow(/unlisted file/);
  });
});

describe("the figure palette patch", () => {
  const palette: [number, number, number][] = [[13, 7, 7], [200, 180, 160]];

  it("inserts the palette uniform and the lookup at three's anchor", () => {
    const material = new MeshStandardMaterial();
    applyFigurePalette(material, palette, 0.25);
    const shader = { uniforms: {} as Record<string, unknown>, fragmentShader: "void main() {\n#include <opaque_fragment>\n}", vertexShader: "" };
    material.onBeforeCompile!(shader as never, {} as never);
    expect(shader.fragmentShader).toContain("uniform vec3 figurePalette[2];");
    expect(shader.fragmentShader).toContain("for (int i = 0; i < 2; i++)");
    expect(shader.fragmentShader).toContain("0.2500");
    expect(Object.keys(shader.uniforms)).toContain("figurePalette");
    expect(material.customProgramCacheKey()).toBe("figure-palette-2-0.25");
  });

  it("refuses to build if the anchor has moved", () => {
    const material = new MeshStandardMaterial();
    applyFigurePalette(material, palette, 0.25);
    const shader = { uniforms: {}, fragmentShader: "void main() {}", vertexShader: "" };
    expect(() => material.onBeforeCompile!(shader as never, {} as never)).toThrow(/anchor the figure palette patches/);
  });
});
