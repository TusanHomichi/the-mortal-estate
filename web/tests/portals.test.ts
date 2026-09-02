import { describe, expect, it } from "vitest";
import type { FeelSpace } from "../src/feelTypes";
import { portalLandingFor } from "../src/space/portals";

const space = {
  portals: [
    { cell: [2, 1], to: { space: "inside", cell: [4, 3] } },
  ],
} as FeelSpace;

describe("portal landing", () => {
  it("uses a portal only when it is the committed route's last square", () => {
    expect(portalLandingFor(space, [{ i: 1, j: 1 }, { i: 2, j: 1 }])).toEqual({
      space: "inside",
      cell: [4, 3],
    });
    expect(portalLandingFor(space, [{ i: 2, j: 1 }, { i: 3, j: 1 }])).toBeNull();
  });
});
