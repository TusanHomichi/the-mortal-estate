import { describe, expect, it } from "vitest";
import { buildWindWeight } from "../src/windWeight";

function syntheticCard(width = 9, height = 9): Uint8ClampedArray {
  return new Uint8ClampedArray(width * height * 4);
}

function fillPixel(
  data: Uint8ClampedArray,
  width: number,
  x: number,
  y: number,
  colour: readonly [number, number, number, number],
): void {
  const offset = (y * width + x) * 4;
  data.set(colour, offset);
}

describe("art-derived wind weight", () => {
  it("gives a high green foliage cluster approximately full weight", () => {
    const width = 9;
    const height = 9;
    const data = syntheticCard(width, height);
    for (let y = 0; y <= 4; y += 1) {
      for (let x = 0; x < width; x += 1) fillPixel(data, width, x, y, [38, 132, 52, 255]);
    }
    const weight = buildWindWeight({ width, height, data }, "tree");
    expect(weight[1 * width + 4]! / 255).toBeGreaterThan(0.85);
  });

  it("keeps low brown bark still", () => {
    const width = 9;
    const height = 9;
    const data = syntheticCard(width, height);
    fillPixel(data, width, 4, 7, [101, 72, 43, 255]);
    const weight = buildWindWeight({ width, height, data }, "tree");
    expect(weight[7 * width + 4]).toBe(0);
  });

  it("fixes green pixels at the card base", () => {
    const width = 9;
    const height = 9;
    const data = syntheticCard(width, height);
    for (let x = 0; x < width; x += 1) fillPixel(data, width, x, 8, [38, 132, 52, 255]);
    const weight = buildWindWeight({ width, height, data }, "tree");
    expect(weight[8 * width + 4]).toBe(0);
  });

  it("gives bare twigs a restrained height-weighted stir", () => {
    const width = 3;
    const height = 3;
    const data = syntheticCard(width, height);
    fillPixel(data, width, 1, 0, [92, 78, 62, 255]);
    fillPixel(data, width, 1, 2, [92, 78, 62, 255]);
    const weight = buildWindWeight({ width, height, data }, "tree_bare");
    expect(weight[1]! / 255).toBeCloseTo(0.15, 2);
    expect(weight[7]).toBe(0);
  });
});
