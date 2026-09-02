import { describe, expect, it } from "vitest";
import { parseFeelManifest, verifySha256 } from "../src/manifest";
import {
  REQUIRED_PROPS,
  REQUIRED_ROOFS,
  REQUIRED_TERRAIN,
  REQUIRED_WALLS,
} from "../src/feelTypes";

const digest = "0".repeat(64);
const row = { file: "synthetic.png", sha256: digest };

function requiredRows(names: readonly string[]): Record<string, typeof row> {
  return Object.fromEntries(names.map((name) => [name, row]));
}

function validManifest(): Record<string, unknown> {
  return {
    schema_version: 2,
    assets: {
      terrain: requiredRows(REQUIRED_TERRAIN),
      walls: requiredRows(REQUIRED_WALLS),
      props: requiredRows(REQUIRED_PROPS),
      roofs: requiredRows(REQUIRED_ROOFS),
    },
    start: { space: "room", cell: [1, 1] },
    spaces: {
      room: {
        grid_extents: { i: 3, j: 3 },
        cells: Array.from({ length: 9 }, (_, index) => ({
          i: index % 3,
          j: Math.floor(index / 3),
          material: "grass",
        })),
        wall_runs: [
          { axis: "x", start: [-0.5, 0.5], cells: 3, door_interval: [1.15, 1.85] },
        ],
        roofs: [],
        props: [],
        light_sources: { lantern_glass: null, candles: [[0, 0.5, 0]] },
        weather: false,
        portals: [],
      },
    },
  };
}

describe("candidate feel manifest", () => {
  it("parses the exact schema-2 spaces packet", () => {
    const parsed = parseFeelManifest(validManifest());
    expect(parsed.schema_version).toBe(2);
    expect(parsed.start).toEqual({ space: "room", cell: [1, 1] });
    expect(parsed.spaces.room?.light_sources.lantern_glass).toBeNull();
  });

  it("accepts a listed additional prop kind with mirror", () => {
    const planted = validManifest() as {
      assets: { props: Record<string, unknown> };
      spaces: { room: { props: unknown[] } };
    };
    planted.assets.props.tree_rare = { file: "prop-tree-rare.png", sha256: digest };
    planted.spaces.room.props.push({
      kind: "tree_rare",
      cell_anchor: [0, 0],
      nominal_height: 1.8,
      sway: true,
      mirror: true,
      facing: "view",
    });

    const parsed = parseFeelManifest(planted);
    expect(parsed.assets.props.tree_rare).toEqual({
      file: "prop-tree-rare.png",
      sha256: digest,
    });
    expect(parsed.spaces.room?.props.at(-1)).toMatchObject({ kind: "tree_rare", mirror: true });
  });

  it("refuses schema 1 by name", () => {
    const retired = validManifest();
    retired.schema_version = 1;
    expect(() => parseFeelManifest(retired)).toThrow(/schema 1 is retired and refused/);
  });

  it("refuses a portal whose source is not a door tile", () => {
    const packet = validManifest() as {
      spaces: { room: { portals: unknown[] } };
    };
    packet.spaces.room.portals = [
      { cell: [0, 0], to: { space: "room", cell: [1, 1] } },
    ];
    expect(() => parseFeelManifest(packet)).toThrow(/portal is not a door tile: room\/0,0/);
  });

  it("refuses a portal target naming an absent space", () => {
    const packet = validManifest() as {
      spaces: { room: { portals: unknown[] } };
    };
    packet.spaces.room.portals = [
      { cell: [1, 0], to: { space: "cellar", cell: [1, 1] } },
    ];
    expect(() => parseFeelManifest(packet)).toThrow(/names an absent space: cellar/);
  });

  it("refuses a portal target that is blocked", () => {
    const packet = validManifest() as {
      spaces: { room: { portals: unknown[]; props: unknown[] } };
    };
    packet.spaces.room.props.push({
      kind: "hearth",
      cell_anchor: [2, 2],
      nominal_height: 1,
      sway: false,
      mirror: false,
      facing: "view",
    });
    packet.spaces.room.portals = [
      { cell: [1, 0], to: { space: "room", cell: [2, 2] } },
    ];
    expect(() => parseFeelManifest(packet)).toThrow(/portal target cell is not walkable/);
  });

  it("refuses a placement naming an unlisted prop kind", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "toString",
      cell_anchor: [0, 0],
      nominal_height: 1,
      sway: false,
      mirror: false,
      facing: "view",
    });
    expect(() => parseFeelManifest(planted)).toThrow(/unlisted kind: toString/);
  });

  it("refuses a caretaker placement because start owns it", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "caretaker",
      cell_anchor: [1, 1],
      nominal_height: 1.38,
      sway: false,
      mirror: false,
      facing: "view",
    });
    expect(() => parseFeelManifest(planted)).toThrow(/start places the caretaker/);
  });

  it("refuses unknown fields instead of silently adapting", () => {
    const planted = validManifest();
    planted.compatibility = true;
    expect(() => parseFeelManifest(planted)).toThrow(/unknown or missing top-level fields/);
  });

  it("refuses a prop placement without facing", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "hearth",
      cell_anchor: [1, 1],
      nominal_height: 1.6,
      sway: false,
      mirror: false,
    });
    expect(() => parseFeelManifest(planted)).toThrow(/prop placement is invalid/);
  });

  it("refuses a prop placement naming an unknown facing", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "hearth",
      cell_anchor: [1, 1],
      nominal_height: 1.6,
      sway: false,
      mirror: false,
      facing: "camera-ish",
    });
    expect(() => parseFeelManifest(planted)).toThrow(/unknown facing: camera-ish/);
  });

  it("refuses a digest mismatch", async () => {
    const bytes = new TextEncoder().encode("candidate bytes").buffer;
    await expect(verifySha256(bytes, digest)).rejects.toThrow(/digest does not match/);
  });

  it("accepts a matching SHA-256", async () => {
    const bytes = new TextEncoder().encode("abc").buffer;
    await expect(
      verifySha256(bytes, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
    ).resolves.toBeUndefined();
  });
});
