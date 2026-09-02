import { describe, expect, it } from "vitest";
import { createFeelCamera } from "../src/camera";
import { cellUnderPointer } from "../src/walk/pointer";

describe("orthographic pointer unprojection", () => {
  it("the centre of the viewport maps to the camera's ground target cell", () => {
    const camera = createFeelCamera(1280, 800, { i: 12, j: 9 });
    const canvas = {
      getBoundingClientRect: () => ({ left: 0, top: 0, width: 1280, height: 800 }),
    };
    expect(cellUnderPointer(camera, canvas, 640, 400, { i: 12, j: 9 })).toEqual({ i: 3, j: 2 });
  });
});
