import { describe, expect, it } from "vitest";
import { createFeelCamera } from "../src/camera";
import { cellUnderPointer } from "../src/walk/pointer";

describe("orthographic pointer unprojection", () => {
  it("the centre ray reaches the ground below the elevated focus", () => {
    const camera = createFeelCamera(1280, 800, { i: 13, j: 11 });
    const canvas = {
      getBoundingClientRect: () => ({ left: 0, top: 0, width: 1280, height: 800 }),
    };
    expect(cellUnderPointer(camera, canvas, 640, 400, { i: 30, j: 22 })).toEqual({
      i: 12,
      j: 10,
    });
  });
});
