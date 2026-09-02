import { describe, expect, it } from "vitest";
import { parseFeelManifest, verifySha256 } from "../src/manifest";
import { REQUIRED_PROPS, REQUIRED_WALLS } from "../src/feelTypes";

const digest = "0".repeat(64);
const row = { file: "synthetic.png", sha256: digest };

function validManifest(): unknown {
  return {
    schema_version: 1,
    assets: {
      terrain: { grass: row },
      walls: Object.fromEntries(REQUIRED_WALLS.map((name) => [name, row])),
      props: Object.fromEntries(REQUIRED_PROPS.map((name) => [name, row])),
    },
    layout: {
      grid_extents: { i: 1, j: 1 },
      cells: [{ i: 0, j: 0, material: "grass" }],
      wall_runs: [{ axis: "x", start: [0, 0], cells: 1, door_interval: null }],
      props: [
        {
          kind: "caretaker",
          cell_anchor: [0, 0],
          nominal_height: 1.2,
          sway: false,
          mirror: false,
        },
      ],
      light_sources: { lantern_glass: [0, 1, 0], candles: [[0, 0.5, 0]] },
    },
  };
}

describe("candidate feel manifest", () => {
  it("parses the exact packet schema", () => {
    const parsed = parseFeelManifest(validManifest());
    expect(parsed.schema_version).toBe(1);
    expect(parsed.layout.cells).toEqual([{ i: 0, j: 0, material: "grass" }]);
  });

  it("accepts a listed additional prop kind with a valid asset row", () => {
    const planted = validManifest() as {
      assets: { props: Record<string, unknown> };
      layout: { props: unknown[] };
    };
    planted.assets.props.tree_slim = { file: "prop-tree-slim.png", sha256: digest };
    planted.layout.props.push({
      kind: "tree_slim",
      cell_anchor: [0, 0],
      nominal_height: 1.8,
      sway: true,
      mirror: true,
    });

    const parsed = parseFeelManifest(planted);
    expect(parsed.assets.props.tree_slim).toEqual({
      file: "prop-tree-slim.png",
      sha256: digest,
    });
    expect(parsed.layout.props.at(-1)).toMatchObject({ kind: "tree_slim", mirror: true });
  });

  it("refuses a placement naming an unlisted prop kind", () => {
    const planted = validManifest() as { layout: { props: Record<string, unknown>[] } };
    planted.layout.props[0]!.kind = "toString";

    expect(() => parseFeelManifest(planted)).toThrow(/unlisted kind: toString/);
  });

  it("refuses an inherited object name as an unlisted terrain asset", () => {
    const planted = validManifest() as { layout: { cells: Record<string, unknown>[] } };
    planted.layout.cells[0]!.material = "toString";

    expect(() => parseFeelManifest(planted)).toThrow(/unknown material/);
  });

  it("refuses a placement without mirror", () => {
    const planted = validManifest() as { layout: { props: Record<string, unknown>[] } };
    delete planted.layout.props[0]!.mirror;

    expect(() => parseFeelManifest(planted)).toThrow(/prop placement is invalid/);
  });

  it("refuses unknown fields instead of silently adapting", () => {
    const planted = validManifest() as Record<string, unknown>;
    planted.compatibility = true;
    expect(() => parseFeelManifest(planted)).toThrow(/unknown or missing top-level fields/);
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
