import { describe, expect, it } from "vitest";
import type { FixturePlacement } from "../src/feelTypes";
import {
  buildHearthGeometry,
  HEARTH_PROFILE,
  hearthFireAnchor,
  hearthLightPosition,
} from "../src/hearthGeometry";
import { WALL_PROFILE } from "../src/wallGeometry";

const NORTH: FixturePlacement = { kind: "hearth", cell: [4, 1], against: "north" };

function bounds(positions: readonly number[]): {
  minX: number;
  maxX: number;
  minY: number;
  maxY: number;
  minZ: number;
  maxZ: number;
} {
  const xs: number[] = [];
  const ys: number[] = [];
  const zs: number[] = [];
  for (let index = 0; index < positions.length; index += 3) {
    xs.push(positions[index]!);
    ys.push(positions[index + 1]!);
    zs.push(positions[index + 2]!);
  }
  return {
    minX: Math.min(...xs),
    maxX: Math.max(...xs),
    minY: Math.min(...ys),
    maxY: Math.max(...ys),
    minZ: Math.min(...zs),
    maxZ: Math.max(...zs),
  };
}

describe("hearth fixture geometry", () => {
  it("puts the breast back on the north wall line and reaches the wall cap", () => {
    const breast = buildHearthGeometry([NORTH]).find((part) => part.label === "breast")!;
    const breastBounds = bounds(breast.geometry.positions);
    expect(breastBounds.minZ).toBeCloseTo(NORTH.cell[1] - 0.5);
    expect(breastBounds.maxZ).toBeCloseTo(NORTH.cell[1] - 0.5 + HEARTH_PROFILE.breastDepth);
    expect(breastBounds.minX).toBeCloseTo(NORTH.cell[0] - 0.5);
    expect(breastBounds.maxX).toBeCloseTo(NORTH.cell[0] + 0.5);
    expect(breastBounds.maxY).toBe(WALL_PROFILE.capTop);
  });

  it("builds the firebox as a real inset behind the breast front", () => {
    const parts = buildHearthGeometry([NORTH]);
    const breastBounds = bounds(parts.find((part) => part.label === "breast")!.geometry.positions);
    const fireboxBounds = bounds(parts.find((part) => part.label === "firebox")!.geometry.positions);
    expect(fireboxBounds.maxZ).toBeCloseTo(breastBounds.maxZ);
    expect(fireboxBounds.minZ).toBeCloseTo(breastBounds.maxZ - HEARTH_PROFILE.fireboxRecess);
    expect(fireboxBounds.minY).toBe(HEARTH_PROFILE.fireboxSill);
    expect(fireboxBounds.maxY).toBe(
      HEARTH_PROFILE.fireboxSill + HEARTH_PROFILE.fireboxHeight,
    );
    expect(hearthFireAnchor(NORTH).position[2]).toBeCloseTo(
      breastBounds.maxZ - HEARTH_PROFILE.fireFrontInset,
    );
  });

  it("overhangs the breast with a wider timber mantel", () => {
    const parts = buildHearthGeometry([NORTH]);
    const breastBounds = bounds(parts.find((part) => part.label === "breast")!.geometry.positions);
    const mantelBounds = bounds(parts.find((part) => part.label === "mantel")!.geometry.positions);
    expect(mantelBounds.minX).toBeCloseTo(NORTH.cell[0] - HEARTH_PROFILE.mantelWidth / 2);
    expect(mantelBounds.maxX).toBeCloseTo(NORTH.cell[0] + HEARTH_PROFILE.mantelWidth / 2);
    expect(mantelBounds.maxZ - breastBounds.maxZ).toBeCloseTo(HEARTH_PROFILE.mantelOverhang);
    expect(mantelBounds.minY).toBe(HEARTH_PROFILE.mantelUnderside);
  });

  it("emits two fieldstone parts, one dark inset, and one timber part per hearth", () => {
    const parts = buildHearthGeometry([NORTH, { kind: "hearth", cell: [2, 3], against: "west" }]);
    expect(parts.filter((part) => part.material === "fieldstone")).toHaveLength(4);
    expect(parts.filter((part) => part.material === "fieldstone_dark")).toHaveLength(2);
    expect(parts.filter((part) => part.material === "post")).toHaveLength(2);
  });

  it("puts the fire light in front of the north breast instead of inside it", () => {
    const breast = buildHearthGeometry([NORTH]).find((part) => part.label === "breast")!;
    const breastBounds = bounds(breast.geometry.positions);
    const light = hearthLightPosition(NORTH);

    expect(light).toEqual([
      NORTH.cell[0],
      HEARTH_PROFILE.lightHeight,
      NORTH.cell[1] - 0.5 + HEARTH_PROFILE.breastDepth + HEARTH_PROFILE.lightFrontOffset,
    ]);
    expect(light[2]).toBeGreaterThan(breastBounds.maxZ);
  });

  it("rotates the same kit to the west wall and keeps its light in front", () => {
    const west: FixturePlacement = { kind: "hearth", cell: [2, 3], against: "west" };
    const breast = buildHearthGeometry([west]).find((part) => part.label === "breast")!;
    const breastBounds = bounds(breast.geometry.positions);
    expect(breastBounds.minX).toBeCloseTo(west.cell[0] - 0.5);
    expect(breastBounds.maxX).toBeCloseTo(west.cell[0] - 0.5 + HEARTH_PROFILE.breastDepth);
    expect(hearthLightPosition(west)).toEqual([
      west.cell[0] - 0.5 + HEARTH_PROFILE.breastDepth + HEARTH_PROFILE.lightFrontOffset,
      HEARTH_PROFILE.lightHeight,
      west.cell[1],
    ]);
    expect(hearthLightPosition(west)[0]).toBeGreaterThan(breastBounds.maxX);
  });
});
