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
    schema_version: 3,
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
        fixtures: [],
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
    expect(parsed.schema_version).toBe(3);
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
      elevation: 0,
      card_height: 1.8,
      sway: true,
      mirror: true,
      facing: "view",
    });

    const parsed = parseFeelManifest(planted);
    expect(parsed.assets.props.tree_rare).toEqual({
      file: "prop-tree-rare.png",
      sha256: digest,
      normal: null,
    });
    expect(parsed.spaces.room?.props.at(-1)).toMatchObject({ kind: "tree_rare", mirror: true });
  });

  it("accepts a prop card's normal sheet beside its colour sheet", () => {
    const planted = validManifest() as { assets: { props: Record<string, unknown> } };
    planted.assets.props.caretaker = {
      file: "prop-caretaker.png",
      sha256: digest,
      normal: { file: "prop-caretaker-normal.png", sha256: "1".repeat(64) },
    };

    const parsed = parseFeelManifest(planted);
    expect(parsed.assets.props.caretaker).toEqual({
      file: "prop-caretaker.png",
      sha256: digest,
      normal: { file: "prop-caretaker-normal.png", sha256: "1".repeat(64) },
    });
  });

  it("refuses a normal sheet on anything but a prop card", () => {
    const planted = validManifest() as { assets: { terrain: Record<string, unknown> } };
    planted.assets.terrain.grass = {
      ...row,
      normal: { file: "terrain-grass-normal.png", sha256: digest },
    };
    expect(() => parseFeelManifest(planted)).toThrow(/only a prop card may/);
  });

  it("refuses a normal sheet with unknown fields or naming the colour sheet", () => {
    const withExtra = validManifest() as { assets: { props: Record<string, unknown> } };
    withExtra.assets.props.caretaker = {
      ...row,
      normal: { file: "prop-caretaker-normal.png", sha256: digest, scale: 1 },
    };
    expect(() => parseFeelManifest(withExtra)).toThrow(/prop normal sheet has unknown or missing fields/);

    const sameFile = validManifest() as { assets: { props: Record<string, unknown> } };
    sameFile.assets.props.caretaker = { ...row, normal: { ...row } };
    expect(() => parseFeelManifest(sameFile)).toThrow(/names its own colour sheet/);

    const nullNormal = validManifest() as { assets: { props: Record<string, unknown> } };
    nullNormal.assets.props.caretaker = { ...row, normal: null };
    expect(() => parseFeelManifest(nullNormal)).toThrow(/prop normal sheet has unknown or missing fields/);
  });

  it("refuses schema 1 by name", () => {
    const retired = validManifest();
    retired.schema_version = 1;
    expect(() => parseFeelManifest(retired)).toThrow(/schema 1 is retired and refused/);
  });

  it("refuses the retired schema 2 by name", () => {
    const retired = validManifest();
    retired.schema_version = 2;
    expect(() => parseFeelManifest(retired)).toThrow(/schema 2 is retired and refused/);
  });

  const treePlacement = (): Record<string, unknown> => ({
    kind: "tree",
    cell_anchor: [2, 2],
    elevation: 0,
    card_height: 3.2,
    sway: true,
    mirror: false,
    facing: "view",
  });

  it("refuses a placement that still carries the retired nominal_height key", () => {
    const stale = validManifest() as { spaces: { room: { props: Record<string, unknown>[] } } };
    const placement = treePlacement();
    placement.nominal_height = placement.card_height;
    delete placement.card_height;
    stale.spaces.room.props.push(placement);
    expect(() => parseFeelManifest(stale)).toThrow(/prop placement is invalid/);
  });

  it("accepts a floor-facing card and carries its facing through", () => {
    const flat = validManifest() as { spaces: { room: { props: Record<string, unknown>[] } } };
    flat.spaces.room.props.push({ ...treePlacement(), facing: "floor" });
    const parsed = parseFeelManifest(flat);
    expect(parsed.spaces.room!.props.at(-1)!.facing).toBe("floor");
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

  it("refuses a portal target that is blocked by a fixture", () => {
    const packet = validManifest() as {
      assets: { walls: Record<string, unknown> };
      spaces: { room: { portals: unknown[]; fixtures: unknown[] } };
    };
    packet.assets.walls.fieldstone = row;
    packet.spaces.room.fixtures.push({ kind: "hearth", cell: [0, 1], against: "north" });
    packet.spaces.room.portals = [
      { cell: [1, 0], to: { space: "room", cell: [0, 1] } },
    ];
    expect(() => parseFeelManifest(packet)).toThrow(/portal target cell is not walkable/);
  });

  it("accepts a hearth fixture against a wall run", () => {
    const packet = validManifest() as {
      assets: { walls: Record<string, unknown> };
      spaces: { room: { fixtures: unknown[] } };
    };
    packet.assets.walls.fieldstone = row;
    packet.spaces.room.fixtures.push({ kind: "hearth", cell: [0, 1], against: "north" });
    expect(parseFeelManifest(packet).spaces.room?.fixtures).toEqual([
      { kind: "hearth", cell: [0, 1], against: "north" },
    ]);
  });

  it("requires fieldstone when a fixture exists", () => {
    const packet = validManifest() as {
      spaces: { room: { fixtures: unknown[] } };
    };
    packet.spaces.room.fixtures.push({ kind: "hearth", cell: [0, 1], against: "north" });
    expect(() => parseFeelManifest(packet)).toThrow(/missing fieldstone required by fixtures/);
  });

  it("refuses an unknown fixture kind", () => {
    const packet = validManifest() as {
      spaces: { room: { fixtures: unknown[] } };
    };
    packet.spaces.room.fixtures.push({ kind: "fountain", cell: [1, 1], against: "north" });
    expect(() => parseFeelManifest(packet)).toThrow(/fixture is invalid/);
  });

  it("refuses an unknown fixture wall direction", () => {
    const packet = validManifest() as {
      spaces: { room: { fixtures: unknown[] } };
    };
    packet.spaces.room.fixtures.push({ kind: "hearth", cell: [0, 1], against: "south" });
    expect(() => parseFeelManifest(packet)).toThrow(/fixture is invalid/);
  });

  it("refuses a fixture against an absent wall line", () => {
    const packet = validManifest() as {
      assets: { walls: Record<string, unknown> };
      spaces: { room: { fixtures: unknown[] } };
    };
    packet.assets.walls.fieldstone = row;
    packet.spaces.room.fixtures.push({ kind: "hearth", cell: [2, 2], against: "west" });
    expect(() => parseFeelManifest(packet)).toThrow(/has no west wall run on its line/);
  });

  it("refuses a placement naming an unlisted prop kind", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "toString",
      cell_anchor: [0, 0],
      elevation: 0,
      card_height: 1,
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
      elevation: 0,
      card_height: 1.38,
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
      kind: "tree",
      cell_anchor: [1, 1],
      elevation: 0,
      card_height: 1.6,
      sway: false,
      mirror: false,
    });
    expect(() => parseFeelManifest(planted)).toThrow(/prop placement is invalid/);
  });

  it("refuses a prop placement without elevation by name", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "tree",
      cell_anchor: [1, 1],
      card_height: 1.6,
      sway: false,
      mirror: false,
      facing: "view",
    });
    expect(() => parseFeelManifest(planted)).toThrow(/without elevation is refused/);
  });

  it("refuses a non-finite or out-of-range prop elevation", () => {
    for (const elevation of [Number.NaN, Number.POSITIVE_INFINITY, -0.01, 6.01]) {
      const planted = validManifest() as {
        spaces: { room: { props: Record<string, unknown>[] } };
      };
      planted.spaces.room.props.push({
        kind: "tree",
        cell_anchor: [1, 1],
        elevation,
        card_height: 1.6,
        sway: false,
        mirror: false,
        facing: "view",
      });
      expect(() => parseFeelManifest(planted)).toThrow(/prop placement is invalid/);
    }
  });

  it("refuses a prop placement naming an unknown facing", () => {
    const planted = validManifest() as {
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.spaces.room.props.push({
      kind: "tree",
      cell_anchor: [1, 1],
      elevation: 0,
      card_height: 1.6,
      sway: false,
      mirror: false,
      facing: "camera-ish",
    });
    expect(() => parseFeelManifest(planted)).toThrow(/unknown facing: camera-ish/);
  });

  it("refuses a retired hearth prop placement even if its asset is listed", () => {
    const planted = validManifest() as {
      assets: { props: Record<string, unknown> };
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    planted.assets.props.hearth = row;
    planted.spaces.room.props.push({
      kind: "hearth",
      cell_anchor: [1, 1],
      elevation: 0,
      card_height: 1.6,
      sway: false,
      mirror: false,
      facing: "+z",
    });
    expect(() => parseFeelManifest(planted)).toThrow(/hearth prop placement is retired and refused/);
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
