import { describe, expect, it } from "vitest";
import type { WallRun } from "../src/feelTypes";
import { buildWallProfile, WALL_PROFILE } from "../src/wallGeometry";

const runs: WallRun[] = [
  { axis: "z", start: [0.5, 0.5], cells: 7, door_interval: null },
  { axis: "x", start: [0.5, 0.5], cells: 8, door_interval: [2.15, 2.85] },
];

function coordinates(positions: number[], axis: 0 | 1 | 2): number[] {
  return positions.filter((_value, index) => index % 3 === axis);
}

describe("v24 wall profile geometry", () => {
  it("builds every requested member as indexed positions and UVs", () => {
    const parts = buildWallProfile(runs);
    expect(parts.length).toBeGreaterThan(30);
    for (const part of parts) {
      expect(part.geometry.positions.length % 3, part.label).toBe(0);
      expect(part.geometry.uvs.length, part.label).toBe((part.geometry.positions.length / 3) * 2);
      expect(part.geometry.indices.length % 3, part.label).toBe(0);
    }
    expect(parts.some((part) => part.label === "corner-post")).toBe(true);
    expect(parts.filter((part) => part.label.startsWith("post-"))).toHaveLength(15);
    expect(parts.filter((part) => part.label.startsWith("brace-"))).toHaveLength(5);
  });

  it("keeps the ruled vertical courses and thickness in world units", () => {
    const parts = buildWallProfile(runs);
    const plinth = parts.find((part) => part.label === "plinth-z-0")!;
    expect(Math.min(...coordinates(plinth.geometry.positions, 1))).toBe(0);
    expect(Math.max(...coordinates(plinth.geometry.positions, 1))).toBe(WALL_PROFILE.plinthTop);
    expect(
      Math.max(...coordinates(plinth.geometry.positions, 0)) -
        Math.min(...coordinates(plinth.geometry.positions, 0)),
    ).toBeCloseTo(WALL_PROFILE.thickness);
    const cap = parts.find((part) => part.label === "cap-front-z")!;
    expect(Math.min(...coordinates(cap.geometry.positions, 1))).toBe(WALL_PROFILE.capBottom);
    expect(Math.max(...coordinates(cap.geometry.positions, 1))).toBe(WALL_PROFILE.capTop);
  });

  it("builds the 0.70 by 1.60 door quad and its lintel", () => {
    const parts = buildWallProfile(runs);
    const door = parts.find((part) => part.label === "door-x")!;
    const xs = coordinates(door.geometry.positions, 0);
    const ys = coordinates(door.geometry.positions, 1);
    expect(Math.max(...xs) - Math.min(...xs)).toBeCloseTo(WALL_PROFILE.doorWidth);
    expect(Math.max(...ys) - Math.min(...ys)).toBeCloseTo(WALL_PROFILE.doorHeight);
    expect(parts.some((part) => part.label === "door-lintel-x")).toBe(true);
  });

  it("carries world-run U coordinates across split wall segments", () => {
    const parts = buildWallProfile(runs);
    const rightPlinth = parts.find((part) => part.label === "plinth-x-2.85")!;
    expect(Math.min(...rightPlinth.geometry.uvs)).toBeCloseTo(0);
    expect(Math.max(...rightPlinth.geometry.uvs)).toBeCloseTo(2);
    expect(rightPlinth.geometry.uvs).toContain(2.85 / 4);
    expect(rightPlinth.geometry.uvs).toContain(8 / 4);
  });
});
