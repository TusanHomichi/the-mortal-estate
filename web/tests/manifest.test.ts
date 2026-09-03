import { describe, expect, it } from "vitest";
import { FIGURE_PALETTE_MAX, POINT_LIGHTS_MAX, parseFeelManifest, verifySha256 } from "../src/manifest";
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

function figureRow(): Record<string, unknown> {
  return {
    rig: { file: "figure.gltf", sha256: digest },
    sidecars: [{ file: "figure.bin", sha256: digest }, { file: "figure-base.png", sha256: digest }],
    clips: { file: "clips.glb", sha256: digest },
    parts: [{ file: "part-body.gltf", sha256: digest, sidecars: [{ file: "part-body.bin", sha256: digest }] }],
    palette: [[13, 7, 7], [200, 180, 160]],
    rim: 0.25,
    idle: "Idle_Loop",
    gait: { walk: "Walk_Loop", run: "Jog_Fwd_Loop", sprint: "Sprint_Loop" },
  };
}

function validManifest(): Record<string, unknown> {
  return {
    schema_version: 5,
    assets: {
      terrain: requiredRows(REQUIRED_TERRAIN),
      walls: requiredRows(REQUIRED_WALLS),
      props: requiredRows(REQUIRED_PROPS),
      roofs: requiredRows(REQUIRED_ROOFS),
    },
    figures: { caretaker: figureRow() },
    caretaker: { figure: "caretaker" },
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
  it("parses the exact schema-5 spaces packet", () => {
    const parsed = parseFeelManifest(validManifest());
    expect(parsed.schema_version).toBe(5);
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
      flat: false,
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
      flat: false,
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

  it("refuses the retired schemas 2, 3, and 4 by name", () => {
    for (const version of [2, 3, 4]) {
      const retired = validManifest();
      retired.schema_version = version;
      expect(() => parseFeelManifest(retired)).toThrow(new RegExp(`schema ${version} is retired and refused`));
    }
  });

  it("carries a figure through with its palette, rim, and idle clip", () => {
    const parsed = parseFeelManifest(validManifest());
    expect(parsed.caretaker).toEqual({ figure: "caretaker" });
    expect(parsed.figures.caretaker!.palette).toEqual([[13, 7, 7], [200, 180, 160]]);
    expect(parsed.figures.caretaker!.rim).toBe(0.25);
    expect(parsed.figures.caretaker!.idle).toBe("Idle_Loop");
    expect(parsed.figures.caretaker!.gait).toEqual({ walk: "Walk_Loop", run: "Jog_Fwd_Loop", sprint: "Sprint_Loop" });
    expect(parsed.figures.caretaker!.parts[0]!.sidecars[0]!.file).toBe("part-body.bin");
  });

  it("refuses a figure that does not name all three gait clips", () => {
    const lame = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    lame.figures.caretaker.gait = { walk: "Walk_Loop", run: "" , sprint: "Sprint_Loop" };
    expect(() => parseFeelManifest(lame)).toThrow(/walk, run, and sprint clips/);
    const partial = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    partial.figures.caretaker.gait = { walk: "Walk_Loop" };
    expect(() => parseFeelManifest(partial)).toThrow(/walk, run, and sprint clips/);
  });

  it("refuses a caretaker that names an unlisted figure", () => {
    const stray = validManifest() as { caretaker: { figure: string } };
    stray.caretaker.figure = "ghost";
    expect(() => parseFeelManifest(stray)).toThrow(/unlisted figure: ghost/);
  });

  it("refuses a figure whose files are not flat, not glTF kinds, or named twice", () => {
    const nested = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    nested.figures.caretaker.rig = { file: "deep/figure.gltf", sha256: digest };
    expect(() => parseFeelManifest(nested)).toThrow(/figure rig is invalid/);
    const wrongKind = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    wrongKind.figures.caretaker.clips = { file: "clips.gltf", sha256: digest };
    expect(() => parseFeelManifest(wrongKind)).toThrow(/must be a \.glb file/);
    const twice = validManifest() as { figures: { caretaker: { sidecars: Record<string, unknown>[] } } };
    twice.figures.caretaker.sidecars.push({ file: "figure.bin", sha256: digest });
    expect(() => parseFeelManifest(twice)).toThrow(/names one file twice/);
  });

  it("refuses a figure palette or rim out of range", () => {
    const pale = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    pale.figures.caretaker.palette = [[0, 0, 300], [1, 1, 1]];
    expect(() => parseFeelManifest(pale)).toThrow(/palette is invalid/);
    const wide = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    wide.figures.caretaker.palette = Array.from({ length: FIGURE_PALETTE_MAX + 1 }, (_, index) => [index % 256, 0, 0]);
    expect(() => parseFeelManifest(wide)).toThrow(/palette is invalid/);
    const widest = validManifest() as { figures: { caretaker: Record<string, unknown> } };
    widest.figures.caretaker.palette = Array.from({ length: FIGURE_PALETTE_MAX }, (_, index) => [index % 256, 0, 0]);
    expect(() => parseFeelManifest(widest)).not.toThrow();
  });

  it("bounds a space's point lights — candles, fixtures, and the lantern together", () => {
    type Lit = { spaces: { room: { light_sources: { lantern_glass: number[] | null; candles: number[][] } } } };
    const over = validManifest() as Lit;
    over.spaces.room.light_sources.candles = Array.from({ length: POINT_LIGHTS_MAX + 1 }, () => [0, 0.5, 0]);
    expect(() => parseFeelManifest(over)).toThrow(/carries 17 point lights .* at most 16/);
    const exact = validManifest() as Lit;
    exact.spaces.room.light_sources.candles = Array.from({ length: POINT_LIGHTS_MAX }, () => [0, 0.5, 0]);
    expect(() => parseFeelManifest(exact)).not.toThrow();
    const withLantern = validManifest() as Lit;
    withLantern.spaces.room.light_sources.candles = Array.from({ length: POINT_LIGHTS_MAX }, () => [0, 0.5, 0]);
    withLantern.spaces.room.light_sources.lantern_glass = [1, 1.5, 1];
    expect(() => parseFeelManifest(withLantern)).toThrow(/carries 17 point lights/);
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

  it("lays a card on the floor only when its row declares it flat", () => {
    const upright = validManifest() as { spaces: { room: { props: Record<string, unknown>[] } } };
    upright.spaces.room.props.push({ ...treePlacement(), facing: "floor" });
    expect(() => parseFeelManifest(upright)).toThrow(/lays tree on the floor, but its card is not declared flat/);

    const flat = validManifest() as {
      assets: { props: Record<string, unknown> };
      spaces: { room: { props: Record<string, unknown>[] } };
    };
    flat.assets.props.rug = { ...row, flat: true };
    flat.spaces.room.props.push({ ...treePlacement(), kind: "rug", sway: false, facing: "floor" });
    const parsed = parseFeelManifest(flat);
    expect(parsed.assets.props.rug!.flat).toBe(true);
    expect(parsed.spaces.room!.props.at(-1)!.facing).toBe("floor");
  });

  it("refuses a flat flag anywhere but a prop row, and a non-boolean one", () => {
    const wall = validManifest() as { assets: { walls: Record<string, unknown> } };
    wall.assets.walls.plaster = { ...row, flat: true };
    expect(() => parseFeelManifest(wall)).toThrow(/declares itself flat; only a prop card may/);
    const odd = validManifest() as { assets: { props: Record<string, unknown> } };
    odd.assets.props.tree = { ...row, flat: "yes" };
    expect(() => parseFeelManifest(odd)).toThrow(/invalid flat flag/);
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
