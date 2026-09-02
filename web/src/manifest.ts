import {
  ASSET_GROUPS,
  REQUIRED_PROPS,
  REQUIRED_WALLS,
  type AssetGroup,
  type AssetRow,
  type FeelManifest,
  type VerifiedAssetPacket,
  type WallRun,
} from "./feelTypes";

const SHA256_PATTERN = /^[0-9a-f]{64}$/;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, expected: readonly string[]): boolean {
  const keys = Object.keys(value);
  return keys.length === expected.length && keys.every((key) => expected.includes(key));
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isInteger(value: unknown): value is number {
  return isFiniteNumber(value) && Number.isInteger(value);
}

function isVector(value: unknown, length: number): value is number[] {
  return Array.isArray(value) && value.length === length && value.every(isFiniteNumber);
}

function isSafePngPath(value: unknown): value is string {
  if (typeof value !== "string" || value.length === 0 || value.includes("\\")) return false;
  if (value.startsWith("/") || !value.toLowerCase().endsWith(".png")) return false;
  const parts = value.split("/");
  return parts.every((part) => part.length > 0 && part !== "." && part !== "..");
}

function parseAssetRow(value: unknown): AssetRow {
  if (!isRecord(value) || !hasExactKeys(value, ["file", "sha256"])) {
    throw new Error("a candidate feel asset row has unknown or missing fields");
  }
  if (!isSafePngPath(value.file) || typeof value.sha256 !== "string") {
    throw new Error("a candidate feel asset row is invalid");
  }
  if (!SHA256_PATTERN.test(value.sha256)) {
    throw new Error("a candidate feel asset digest is invalid");
  }
  return { file: value.file, sha256: value.sha256 };
}

function parseAssetGroup(value: unknown, name: AssetGroup): Record<string, AssetRow> {
  if (!isRecord(value) || Object.keys(value).length === 0) {
    throw new Error(`the candidate feel ${name} group is empty`);
  }
  return Object.fromEntries(
    Object.entries(value).map(([assetName, row]) => [assetName, parseAssetRow(row)]),
  );
}

function parseWallRun(value: unknown): WallRun {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["axis", "start", "cells", "door_interval"]) ||
    (value.axis !== "x" && value.axis !== "z") ||
    !isVector(value.start, 2) ||
    !isInteger(value.cells) ||
    value.cells <= 0
  ) {
    throw new Error("a candidate feel wall run is invalid");
  }
  let door: [number, number] | null = null;
  if (value.door_interval !== null) {
    if (!isVector(value.door_interval, 2)) {
      throw new Error("a candidate feel wall run has an invalid door interval");
    }
    const [start, end] = value.door_interval;
    if (start === undefined || end === undefined || start < 0 || end > value.cells || start >= end) {
      throw new Error("a candidate feel wall run has an invalid door interval");
    }
    door = [start, end];
  }
  return { axis: value.axis, start: [value.start[0]!, value.start[1]!], cells: value.cells, door_interval: door };
}

function parseLayout(value: unknown, manifestAssets: FeelManifest["assets"]): FeelManifest["layout"] {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["grid_extents", "cells", "wall_runs", "props", "light_sources"])
  ) {
    throw new Error("the candidate feel layout has unknown or missing fields");
  }
  const extents = value.grid_extents;
  if (
    !isRecord(extents) ||
    !hasExactKeys(extents, ["i", "j"]) ||
    !isInteger(extents.i) ||
    !isInteger(extents.j) ||
    extents.i <= 0 ||
    extents.j <= 0
  ) {
    throw new Error("the candidate feel grid extents are invalid");
  }
  if (!Array.isArray(value.cells) || value.cells.length !== extents.i * extents.j) {
    throw new Error("the candidate feel material plan must name every cell exactly once");
  }
  const extentI = extents.i;
  const extentJ = extents.j;
  const seen = new Set<string>();
  const cells = value.cells.map((candidate) => {
    if (
      !isRecord(candidate) ||
      !hasExactKeys(candidate, ["i", "j", "material"]) ||
      !isInteger(candidate.i) ||
      !isInteger(candidate.j) ||
      typeof candidate.material !== "string"
    ) {
      throw new Error("a candidate feel cell is invalid");
    }
    const key = `${candidate.i}:${candidate.j}`;
    if (
      candidate.i < 0 ||
      candidate.i >= extentI ||
      candidate.j < 0 ||
      candidate.j >= extentJ ||
      seen.has(key)
    ) {
      throw new Error("the candidate feel material plan has an invalid or duplicate cell");
    }
    if (!(candidate.material in manifestAssets.terrain)) {
      throw new Error("a candidate feel cell names an unknown material");
    }
    seen.add(key);
    return { i: candidate.i, j: candidate.j, material: candidate.material };
  });

  if (!Array.isArray(value.wall_runs) || value.wall_runs.length === 0) {
    throw new Error("the candidate feel layout carries no wall runs");
  }
  const wallRuns = value.wall_runs.map(parseWallRun);

  if (!Array.isArray(value.props) || value.props.length === 0) {
    throw new Error("the candidate feel layout carries no props");
  }
  const props = value.props.map((candidate) => {
    if (
      !isRecord(candidate) ||
      !hasExactKeys(candidate, ["kind", "cell_anchor", "nominal_height", "sway"]) ||
      typeof candidate.kind !== "string" ||
      !(candidate.kind in manifestAssets.props) ||
      !isVector(candidate.cell_anchor, 2) ||
      !isFiniteNumber(candidate.nominal_height) ||
      candidate.nominal_height <= 0 ||
      typeof candidate.sway !== "boolean"
    ) {
      throw new Error("a candidate feel prop placement is invalid");
    }
    return {
      kind: candidate.kind,
      cell_anchor: [candidate.cell_anchor[0]!, candidate.cell_anchor[1]!] as [number, number],
      nominal_height: candidate.nominal_height,
      sway: candidate.sway,
    };
  });

  const lights = value.light_sources;
  if (
    !isRecord(lights) ||
    !hasExactKeys(lights, ["lantern_glass", "candles"]) ||
    !isVector(lights.lantern_glass, 3) ||
    !Array.isArray(lights.candles) ||
    !lights.candles.every((candle) => isVector(candle, 3))
  ) {
    throw new Error("the candidate feel light positions are invalid");
  }

  return {
    grid_extents: { i: extentI, j: extentJ },
    cells,
    wall_runs: wallRuns,
    props,
    light_sources: {
      lantern_glass: [...lights.lantern_glass] as [number, number, number],
      candles: lights.candles.map((candle) => [...candle] as [number, number, number]),
    },
  };
}

export function parseFeelManifest(value: unknown): FeelManifest {
  if (!isRecord(value) || !hasExactKeys(value, ["schema_version", "assets", "layout"])) {
    throw new Error("the candidate feel manifest has unknown or missing top-level fields");
  }
  if (value.schema_version !== 1 || !isRecord(value.assets)) {
    throw new Error("the candidate feel manifest schema version or assets are invalid");
  }
  if (!hasExactKeys(value.assets, ASSET_GROUPS)) {
    throw new Error("the candidate feel asset groups are incomplete");
  }
  const assets = {
    terrain: parseAssetGroup(value.assets.terrain, "terrain"),
    walls: parseAssetGroup(value.assets.walls, "walls"),
    props: parseAssetGroup(value.assets.props, "props"),
  };
  for (const required of REQUIRED_WALLS) {
    if (!(required in assets.walls)) throw new Error(`the candidate feel wall set is missing ${required}`);
  }
  for (const required of REQUIRED_PROPS) {
    if (!(required in assets.props)) throw new Error(`the candidate feel prop set is missing ${required}`);
  }
  return { schema_version: 1, assets, layout: parseLayout(value.layout, assets) };
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function verifySha256(
  bytes: ArrayBuffer,
  expected: string,
  subtle: SubtleCrypto = globalThis.crypto.subtle,
): Promise<void> {
  const digest = await subtle.digest("SHA-256", bytes);
  if (bytesToHex(new Uint8Array(digest)) !== expected) {
    throw new Error("asset digest does not match the manifest");
  }
}

export async function fetchVerifiedAssetPacket(
  prefix = "/feel-assets/",
  fetcher: typeof fetch = fetch,
): Promise<VerifiedAssetPacket> {
  const manifestResponse = await fetcher(`${prefix}feel-manifest.json`, { cache: "no-store" });
  if (!manifestResponse.ok) throw new Error("candidate feel manifest is absent or unreadable");
  let source: unknown;
  try {
    source = await manifestResponse.json();
  } catch {
    throw new Error("candidate feel manifest is not valid JSON");
  }
  const manifest = parseFeelManifest(source);
  const assets = new Map<string, { bytes: ArrayBuffer; file: string }>();
  const verifiedFiles = new Map<string, ArrayBuffer>();
  for (const group of ASSET_GROUPS) {
    for (const [name, row] of Object.entries(manifest.assets[group])) {
      const identity = `${row.file}:${row.sha256}`;
      let bytes = verifiedFiles.get(identity);
      if (bytes === undefined) {
        const response = await fetcher(`${prefix}${row.file}`, { cache: "no-store" });
        if (!response.ok) throw new Error(`${group}/${name}: asset file is missing`);
        bytes = await response.arrayBuffer();
        await verifySha256(bytes, row.sha256);
        verifiedFiles.set(identity, bytes);
      }
      assets.set(`${group}/${name}`, { bytes, file: row.file });
    }
  }
  return { manifest, assets };
}
