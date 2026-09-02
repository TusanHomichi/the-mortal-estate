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

describe("wall profile geometry", () => {
  it("builds every requested member as indexed positions and UVs", () => {
    const parts = buildWallProfile(runs);
    expect(parts.length).toBeGreaterThan(30);
    for (const part of parts) {
      expect(part.geometry.positions.length % 3, part.label).toBe(0);
      expect(part.geometry.uvs.length, part.label).toBe((part.geometry.positions.length / 3) * 2);
      expect(part.geometry.indices.length % 3, part.label).toBe(0);
    }
    expect(parts.some((part) => part.label === "corner-post")).toBe(true);
    expect(parts.find((part) => part.label === "cap-front-z")?.runIndex).toBe(0);
    expect(parts.find((part) => part.label === "cap-front-x")?.runIndex).toBe(1);
    expect(parts.filter((part) => part.label.startsWith("post-"))).toHaveLength(15);
    expect(parts.filter((part) => part.label.startsWith("brace-"))).toHaveLength(5);
  });

  it("keeps the ruled courses and extends thickness into each wall tile", () => {
    const parts = buildWallProfile(runs);
    const plinth = parts.find((part) => part.label === "plinth-z-0")!;
    expect(Math.min(...coordinates(plinth.geometry.positions, 1))).toBe(0);
    expect(Math.max(...coordinates(plinth.geometry.positions, 1))).toBe(WALL_PROFILE.plinthTop);
    expect(
      Math.max(...coordinates(plinth.geometry.positions, 0)) -
        Math.min(...coordinates(plinth.geometry.positions, 0)),
    ).toBeCloseTo(WALL_PROFILE.thickness);
    expect(Math.max(...coordinates(plinth.geometry.positions, 0))).toBe(0.5);
    expect(Math.min(...coordinates(plinth.geometry.positions, 0))).toBeCloseTo(
      0.5 - WALL_PROFILE.thickness,
    );
    const xPlinth = parts.find((part) => part.label === "plinth-x-0")!;
    expect(Math.max(...coordinates(xPlinth.geometry.positions, 2))).toBe(0.5);
    expect(Math.min(...coordinates(xPlinth.geometry.positions, 2))).toBeCloseTo(
      0.5 - WALL_PROFILE.thickness,
    );
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
    const doorZs = coordinates(door.geometry.positions, 2);
    expect(Math.min(...doorZs)).toBeCloseTo(0.502);
    expect(Math.max(...doorZs)).toBeCloseTo(0.502);
    const lintel = parts.find((part) => part.label === "door-lintel-x")!;
    expect(Math.max(...coordinates(lintel.geometry.positions, 2))).toBe(0.5);
    expect(Math.min(...coordinates(lintel.geometry.positions, 2))).toBeCloseTo(
      0.5 - (WALL_PROFILE.thickness + 0.025),
    );

    const zDoorParts = buildWallProfile([
      { axis: "z", start: [0.5, 0.5], cells: 3, door_interval: [1.15, 1.85] },
    ]);
    const zDoor = zDoorParts.find((part) => part.label === "door-z")!;
    const doorXs = coordinates(zDoor.geometry.positions, 0);
    expect(Math.min(...doorXs)).toBeCloseTo(0.502);
    expect(Math.max(...doorXs)).toBeCloseTo(0.502);
    const zLintel = zDoorParts.find((part) => part.label === "door-lintel-z")!;
    expect(Math.max(...coordinates(zLintel.geometry.positions, 0))).toBe(0.5);
    expect(Math.min(...coordinates(zLintel.geometry.positions, 0))).toBeCloseTo(
      0.5 - (WALL_PROFILE.thickness + 0.025),
    );
  });

  it("moves the corner post into the wall-owned corner tile", () => {
    const corner = buildWallProfile(runs).find((part) => part.label === "corner-post")!;
    const xs = coordinates(corner.geometry.positions, 0);
    const zs = coordinates(corner.geometry.positions, 2);
    expect(Math.max(...xs)).toBe(0.5);
    expect(Math.max(...zs)).toBe(0.5);
    expect(Math.min(...xs)).toBeCloseTo(0.5 - WALL_PROFILE.cornerPostWidth);
    expect(Math.min(...zs)).toBeCloseTo(0.5 - WALL_PROFILE.cornerPostWidth);
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
