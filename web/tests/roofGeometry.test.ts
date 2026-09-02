import { describe, expect, it } from "vitest";
import {
  buildRoofGeometry,
  mergeGeometryData,
  ROOF_OVERHANG,
  ROOF_PITCH_RISE_MULTIPLIER,
  ROOF_SHINGLE_SLOPE_UV_SCALE,
  ROOF_SHINGLE_SLOPE_VALUE_MULTIPLIER,
} from "../src/roofGeometry";
import type { RoofPlacement } from "../src/feelTypes";

const roof: RoofPlacement = {
  footprint: { i0: 0, j0: 1, i1: 2, j1: 4 },
  ridge_axis: "x",
  eave_height: 2.2,
  ridge_height: 3.5,
  material: "shingle",
};

function coordinates(positions: readonly number[], axis: number): number[] {
  return positions.filter((_, index) => index % 3 === axis);
}

describe("pitched roof geometry", () => {
  it("builds two tiled slopes at the eaves and ridge with the ruled overhang", () => {
    const parts = buildRoofGeometry(roof);
    const slopes = parts.filter((part) => part.material === "shingle_slope");
    expect(slopes).toHaveLength(2);
    const positions = slopes.flatMap((part) => part.geometry.positions);
    expect(Math.min(...coordinates(positions, 0))).toBeCloseTo(-0.5 - ROOF_OVERHANG);
    expect(Math.max(...coordinates(positions, 0))).toBeCloseTo(2.5 + ROOF_OVERHANG);
    expect(coordinates(positions, 1)).toContain(roof.eave_height);
    expect(coordinates(positions, 1)).toContain(
      roof.eave_height + (roof.ridge_height - roof.eave_height) * ROOF_PITCH_RISE_MULTIPLIER,
    );
    expect(Math.max(...slopes[0]!.geometry.uvs)).toBeGreaterThan(3);
    expect(ROOF_SHINGLE_SLOPE_UV_SCALE).toBe(1.6);
    expect(ROOF_SHINGLE_SLOPE_VALUE_MULTIPLIER).toBe(0.8);
  });

  it("adds the ridge, both eaves, two plaster gables, and four timber rakes", () => {
    const parts = buildRoofGeometry(roof);
    expect(parts.filter((part) => part.material === "shingle_ridge")).toHaveLength(1);
    expect(parts.filter((part) => part.material === "shingle_eave")).toHaveLength(2);
    expect(parts.filter((part) => part.material === "plaster")).toHaveLength(2);
    expect(parts.filter((part) => part.material === "post")).toHaveLength(4);
  });

  it("rotates the same construction for a z-axis ridge", () => {
    const rotated = buildRoofGeometry({ ...roof, ridge_axis: "z" });
    const slopes = rotated.filter((part) => part.material === "shingle_slope");
    const positions = slopes.flatMap((part) => part.geometry.positions);
    expect(Math.min(...coordinates(positions, 2))).toBeCloseTo(0.5 - ROOF_OVERHANG);
    expect(Math.max(...coordinates(positions, 2))).toBeCloseTo(4.5 + ROOF_OVERHANG);
    expect(coordinates(positions, 1)).toContain(
      roof.eave_height + (roof.ridge_height - roof.eave_height) * ROOF_PITCH_RISE_MULTIPLIER,
    );
  });

  it("offsets indices when roof parts are batched", () => {
    const slopes = buildRoofGeometry(roof)
      .filter((part) => part.material === "shingle_slope")
      .map((part) => part.geometry);
    const merged = mergeGeometryData(slopes);
    expect(merged.positions).toHaveLength(
      slopes.reduce((total, geometry) => total + geometry.positions.length, 0),
    );
    expect(Math.max(...merged.indices)).toBe(merged.positions.length / 3 - 1);
  });
});
