import { describe, expect, it } from "vitest";
import { Vector3 } from "three";
import {
  CAMERA_TARGET_HEIGHT,
  cameraFocusFor,
  cameraFollowsCaretaker,
  spaceCameraFocus,
  CAMERA_ZOOM_STEP_RATIO,
  cameraVerticalSize,
  createFeelCamera,
  focusFeelCamera,
  projectedCellDiamondWidth,
  projectedHeightCoverTiles,
} from "../src/camera";
import { WALL_PROFILE } from "../src/wallGeometry";

describe("the ruled feel camera", () => {
  it("projects one cell to a 179.2-pixel-wide diamond at 1280 by 800 (the ruled frame)", () => {
    const camera = createFeelCamera(1280, 800, { i: 5, j: 5 });
    expect(projectedCellDiamondWidth(camera, 1280)).toBeCloseTo(179.2, 6);
  });

  it("one comparison step out widens the frame by the step ratio and keeps the focus centred", () => {
    const ruled = createFeelCamera(1280, 800, { i: 5, j: 5 });
    const stepOut = createFeelCamera(1280, 800, { i: 5, j: 5 }, -1);
    expect(projectedCellDiamondWidth(stepOut, 1280)).toBeCloseTo(179.2 / CAMERA_ZOOM_STEP_RATIO, 6);
    expect(stepOut.top - stepOut.bottom).toBeCloseTo((ruled.top - ruled.bottom) * CAMERA_ZOOM_STEP_RATIO, 9);
    const projected = new Vector3(5, CAMERA_TARGET_HEIGHT, 5).project(stepOut);
    expect(projected.x).toBeCloseTo(0, 12);
    expect(projected.y).toBeCloseTo(0, 12);
    expect(cameraVerticalSize(0)).toBe(cameraVerticalSize(0));
    expect(cameraVerticalSize(1)).toBeCloseTo(cameraVerticalSize(0) / CAMERA_ZOOM_STEP_RATIO, 9);
    expect(() => cameraVerticalSize(4)).toThrow(/outside the comparison range/);
    expect(() => cameraVerticalSize(0.5)).toThrow(/outside the comparison range/);
  });

  it("inside a building the camera belongs to the space; outside it follows the caretaker", () => {
    const interior = { grid_extents: { i: 9, j: 5 }, weather: false };
    const exterior = { grid_extents: { i: 30, j: 22 }, weather: true };
    expect(spaceCameraFocus(interior.grid_extents)).toEqual({ i: 4, j: 2 });
    expect(cameraFollowsCaretaker(interior)).toBe(false);
    expect(cameraFollowsCaretaker(exterior)).toBe(true);
    expect(cameraFocusFor(interior, { i: 4, j: 3 })).toEqual({ i: 4, j: 2 });
    expect(cameraFocusFor(interior, { i: 1, j: 1 })).toEqual({ i: 4, j: 2 });
    expect(cameraFocusFor(exterior, { i: 13, j: 11 })).toEqual({ i: 13, j: 11 });
    const camera = createFeelCamera(1280, 800, spaceCameraFocus(interior.grid_extents));
    const projected = new Vector3(4, CAMERA_TARGET_HEIGHT, 2).project(camera);
    expect(projected.x).toBeCloseTo(0, 12);
    expect(projected.y).toBeCloseTo(0, 12);
  });

  it("the focus cell projects to the viewport centre for any cell", () => {
    const camera = createFeelCamera(1280, 800, { i: 0, j: 0 });
    for (const cell of [
      { i: 0, j: 0 },
      { i: 13, j: 11 },
      { i: 29, j: 21 },
      { i: -7, j: 42 },
    ]) {
      focusFeelCamera(camera, cell);
      const projected = new Vector3(cell.i, CAMERA_TARGET_HEIGHT, cell.j).project(camera);
      expect(projected.x).toBeCloseTo(0, 12);
      expect(projected.y).toBeCloseTo(0, 12);
    }
  });

  it("derives a wall's ground cover from the ruled camera and profile", () => {
    expect(projectedHeightCoverTiles(WALL_PROFILE.capTop)).toBeCloseTo(5.39, 2);
  });
});
