import { describe, expect, it } from "vitest";
import { createFeelCamera, projectedCellDiamondWidth } from "../src/camera";

describe("the ruled feel camera", () => {
  it("projects one cell to a 224-pixel-wide diamond at 1280 by 800", () => {
    const camera = createFeelCamera(1280, 800, { i: 12, j: 9 });
    expect(projectedCellDiamondWidth(camera, 1280)).toBeCloseTo(224, 6);
  });
});
