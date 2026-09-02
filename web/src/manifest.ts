import {
  ASSET_GROUPS,
  REQUIRED_PROPS,
  REQUIRED_ROOFS,
  REQUIRED_TERRAIN,
  REQUIRED_WALLS,
  type AssetFile,
  type AssetGroup,
  type AssetRow,
  type FeelManifest,
  type FeelSpace,
  type FixturePlacement,
  type PortalPlacement,
  type PortalTarget,
  type PropPlacement,
  type RoofPlacement,
  type VerifiedAssetPacket,
  type WallRun,
} from "./feelTypes";
import { wallAndDoorTiles } from "./space/layoutTiles";
import { cellKey, passabilityFrom } from "./walk/layoutPassability";

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

function isIntegerVector(value: unknown, length: number): value is number[] {
  return Array.isArray(value) && value.length === length && value.every(isInteger);
}

function isPropFacing(value: unknown): value is PropPlacement["facing"] {
  return value === "view" || value === "+z" || value === "+x";
}

function isSafePngPath(value: unknown): value is string {
  if (typeof value !== "string" || value.length === 0 || value.includes("\\")) return false;
  if (value.startsWith("/") || !value.toLowerCase().endsWith(".png")) return false;
  const parts = value.split("/");
  return parts.every((part) => part.length > 0 && part !== "." && part !== "..");
}

function parseAssetFile(value: unknown, what: string): AssetFile {
  if (!isRecord(value) || !hasExactKeys(value, ["file", "sha256"])) {
    throw new Error(`a candidate feel ${what} has unknown or missing fields`);
  }
  if (!isSafePngPath(value.file) || typeof value.sha256 !== "string") {
    throw new Error(`a candidate feel ${what} is invalid`);
  }
  if (!SHA256_PATTERN.test(value.sha256)) {
    throw new Error(`a candidate feel ${what} digest is invalid`);
  }
  return { file: value.file, sha256: value.sha256 };
}

function parseAssetRow(value: unknown, group: AssetGroup): AssetRow {
  if (!isRecord(value) || !Object.hasOwn(value, "normal")) {
    return { ...parseAssetFile(value, "asset row"), normal: null };
  }
  if (group !== "props") {
    throw new Error(`a candidate feel ${group} asset row carries a normal sheet; only a prop card may`);
  }
  if (!hasExactKeys(value, ["file", "sha256", "normal"])) {
    throw new Error("a candidate feel prop asset row has unknown or missing fields");
  }
  const colour = parseAssetFile({ file: value.file, sha256: value.sha256 }, "asset row");
  const normal = parseAssetFile(value.normal, "prop normal sheet");
  if (normal.file === colour.file) {
    throw new Error("a candidate feel prop normal sheet names its own colour sheet");
  }
  return { ...colour, normal };
}

function parseAssetGroup(value: unknown, name: AssetGroup): Record<string, AssetRow> {
  if (!isRecord(value) || Object.keys(value).length === 0) {
    throw new Error(`the candidate feel ${name} group is empty`);
  }
  return Object.fromEntries(
    Object.entries(value).map(([assetName, row]) => [assetName, parseAssetRow(row, name)]),
  );
}

function requireAssets(
  assets: FeelManifest["assets"],
  group: AssetGroup,
  names: readonly string[],
): void {
  for (const name of names) {
    if (!Object.hasOwn(assets[group], name)) {
      throw new Error(`the candidate feel ${group} set is missing ${name}`);
    }
  }
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
  return {
    axis: value.axis,
    start: [value.start[0]!, value.start[1]!],
    cells: value.cells,
    door_interval: door,
  };
}

function parseRoof(
  value: unknown,
  extents: { i: number; j: number },
  assets: FeelManifest["assets"],
): RoofPlacement {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["footprint", "ridge_axis", "eave_height", "ridge_height", "material"]) ||
    !isRecord(value.footprint) ||
    !hasExactKeys(value.footprint, ["i0", "j0", "i1", "j1"]) ||
    !isInteger(value.footprint.i0) ||
    !isInteger(value.footprint.j0) ||
    !isInteger(value.footprint.i1) ||
    !isInteger(value.footprint.j1) ||
    (value.ridge_axis !== "x" && value.ridge_axis !== "z") ||
    !isFiniteNumber(value.eave_height) ||
    !isFiniteNumber(value.ridge_height) ||
    value.eave_height <= 0 ||
    value.ridge_height <= value.eave_height ||
    value.material !== "shingle"
  ) {
    throw new Error("a candidate feel roof is invalid");
  }
  const footprint = {
    i0: value.footprint.i0,
    j0: value.footprint.j0,
    i1: value.footprint.i1,
    j1: value.footprint.j1,
  };
  if (
    footprint.i0 < 0 ||
    footprint.j0 < 0 ||
    footprint.i1 < footprint.i0 ||
    footprint.j1 < footprint.j0 ||
    footprint.i1 >= extents.i ||
    footprint.j1 >= extents.j
  ) {
    throw new Error("a candidate feel roof footprint is outside its space");
  }
  for (const suffix of ["slope", "ridge", "eave"] as const) {
    if (!Object.hasOwn(assets.roofs, `${value.material}_${suffix}`)) {
      throw new Error(`a candidate feel roof names an incomplete material: ${value.material}`);
    }
  }
  return {
    footprint,
    ridge_axis: value.ridge_axis,
    eave_height: value.eave_height,
    ridge_height: value.ridge_height,
    material: value.material,
  };
}

function parsePortalTarget(value: unknown, context: string): PortalTarget {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["space", "cell"]) ||
    typeof value.space !== "string" ||
    value.space.length === 0 ||
    !isIntegerVector(value.cell, 2)
  ) {
    throw new Error(`the candidate feel ${context} is invalid`);
  }
  return { space: value.space, cell: [value.cell[0]!, value.cell[1]!] };
}

function parsePortal(value: unknown): PortalPlacement {
  if (!isRecord(value) || !hasExactKeys(value, ["cell", "to"]) || !isIntegerVector(value.cell, 2)) {
    throw new Error("a candidate feel portal is invalid");
  }
  return {
    cell: [value.cell[0]!, value.cell[1]!],
    to: parsePortalTarget(value.to, "portal target"),
  };
}

function wallRunSupportsFixture(
  run: WallRun,
  fixture: FixturePlacement,
): boolean {
  const [i, j] = fixture.cell;
  const axisPosition = fixture.against === "north" ? i : j;
  const runStart = fixture.against === "north" ? run.start[0] : run.start[1];
  const local = axisPosition - runStart;
  const onLine = fixture.against === "north"
    ? run.axis === "x" && run.start[1] === j - 0.5
    : run.axis === "z" && run.start[0] === i - 0.5;
  if (!onLine || local < 0 || local > run.cells) return false;
  return run.door_interval === null ||
    local < run.door_interval[0] || local > run.door_interval[1];
}

function parseFixture(
  value: unknown,
  wallRuns: readonly WallRun[],
  cells: ReadonlySet<string>,
): FixturePlacement {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, ["kind", "cell", "against"]) ||
    value.kind !== "hearth" ||
    !isIntegerVector(value.cell, 2) ||
    (value.against !== "north" && value.against !== "west")
  ) {
    throw new Error("a candidate feel fixture is invalid");
  }
  const fixture: FixturePlacement = {
    kind: "hearth",
    cell: [value.cell[0]!, value.cell[1]!],
    against: value.against,
  };
  if (!cells.has(cellKey({ i: fixture.cell[0], j: fixture.cell[1] }))) {
    throw new Error("a candidate feel fixture cell is outside its space");
  }
  if (!wallRuns.some((run) => wallRunSupportsFixture(run, fixture))) {
    throw new Error(
      `a candidate feel ${fixture.kind} fixture has no ${fixture.against} wall run on its line`,
    );
  }
  return fixture;
}

function parseSpace(
  value: unknown,
  assets: FeelManifest["assets"],
): FeelSpace {
  if (
    !isRecord(value) ||
    !hasExactKeys(value, [
      "grid_extents",
      "cells",
      "wall_runs",
      "roofs",
      "props",
      "fixtures",
      "light_sources",
      "weather",
      "portals",
    ])
  ) {
    throw new Error("a candidate feel space has unknown or missing fields");
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
  const extentI = extents.i;
  const extentJ = extents.j;
  if (!Array.isArray(value.cells) || value.cells.length !== extentI * extentJ) {
    throw new Error("the candidate feel material plan must name every cell exactly once");
  }
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
    if (!Object.hasOwn(assets.terrain, candidate.material)) {
      throw new Error("a candidate feel cell names an unknown material");
    }
    seen.add(key);
    return { i: candidate.i, j: candidate.j, material: candidate.material };
  });
  const cellKeys = new Set(cells.map(cellKey));

  if (!Array.isArray(value.wall_runs)) throw new Error("a candidate feel space has invalid wall runs");
  const wallRuns = value.wall_runs.map(parseWallRun);
  if (!Array.isArray(value.roofs)) throw new Error("a candidate feel space has invalid roofs");
  const roofs = value.roofs.map((roof) => parseRoof(roof, { i: extentI, j: extentJ }, assets));

  if (!Array.isArray(value.props)) throw new Error("a candidate feel space has invalid props");
  const props = value.props.map((candidate) => {
    if (!isRecord(candidate) || !Object.hasOwn(candidate, "elevation")) {
      throw new Error("a candidate feel prop placement without elevation is refused");
    }
    if (
      !hasExactKeys(candidate, [
        "kind",
        "cell_anchor",
        "elevation",
        "nominal_height",
        "sway",
        "mirror",
        "facing",
      ]) ||
      typeof candidate.kind !== "string" ||
      !isVector(candidate.cell_anchor, 2) ||
      !isFiniteNumber(candidate.elevation) ||
      candidate.elevation < 0 ||
      candidate.elevation > 6 ||
      !isFiniteNumber(candidate.nominal_height) ||
      candidate.nominal_height <= 0 ||
      typeof candidate.sway !== "boolean" ||
      typeof candidate.mirror !== "boolean"
    ) {
      throw new Error("a candidate feel prop placement is invalid");
    }
    if (!isPropFacing(candidate.facing)) {
      throw new Error(
        `a candidate feel prop placement names an unknown facing: ${String(candidate.facing)}`,
      );
    }
    if (candidate.kind === "caretaker") {
      throw new Error("a candidate feel caretaker placement is refused; start places the caretaker");
    }
    if (candidate.kind === "hearth") {
      throw new Error("a candidate feel hearth prop placement is retired and refused; use a fixture");
    }
    if (!Object.hasOwn(assets.props, candidate.kind)) {
      throw new Error(`a candidate feel prop placement names an unlisted kind: ${candidate.kind}`);
    }
    return {
      kind: candidate.kind,
      cell_anchor: [candidate.cell_anchor[0]!, candidate.cell_anchor[1]!] as [number, number],
      elevation: candidate.elevation,
      nominal_height: candidate.nominal_height,
      sway: candidate.sway,
      mirror: candidate.mirror,
      facing: candidate.facing,
    };
  });

  if (!Array.isArray(value.fixtures)) {
    throw new Error("a candidate feel space has invalid fixtures");
  }
  const fixtures = value.fixtures.map((fixture) => parseFixture(fixture, wallRuns, cellKeys));
  if (new Set(fixtures.map((fixture) => cellKey({ i: fixture.cell[0], j: fixture.cell[1] }))).size !== fixtures.length) {
    throw new Error("a candidate feel space has duplicate fixture cells");
  }
  if (fixtures.length > 0 && !Object.hasOwn(assets.walls, "fieldstone")) {
    throw new Error("the candidate feel walls set is missing fieldstone required by fixtures");
  }

  const lights = value.light_sources;
  if (
    !isRecord(lights) ||
    !hasExactKeys(lights, ["lantern_glass", "candles"]) ||
    (lights.lantern_glass !== null && !isVector(lights.lantern_glass, 3)) ||
    !Array.isArray(lights.candles) ||
    !lights.candles.every((candle) => isVector(candle, 3))
  ) {
    throw new Error("the candidate feel light positions are invalid");
  }
  if (typeof value.weather !== "boolean" || !Array.isArray(value.portals)) {
    throw new Error("the candidate feel space weather or portals are invalid");
  }
  const portals = value.portals.map(parsePortal);
  if (new Set(portals.map((portal) => `${portal.cell[0]},${portal.cell[1]}`)).size !== portals.length) {
    throw new Error("a candidate feel space has duplicate portal cells");
  }

  return {
    grid_extents: { i: extentI, j: extentJ },
    cells,
    wall_runs: wallRuns,
    roofs,
    props,
    fixtures,
    light_sources: {
      lantern_glass: lights.lantern_glass === null
        ? null
        : [...lights.lantern_glass] as [number, number, number],
      candles: lights.candles.map((candle) => [...candle] as [number, number, number]),
    },
    weather: value.weather,
    portals,
  };
}

function validateLanding(
  spaces: Record<string, FeelSpace>,
  target: PortalTarget,
  context: string,
): void {
  const space = spaces[target.space];
  if (space === undefined) {
    throw new Error(`the candidate feel ${context} names an absent space: ${target.space}`);
  }
  const key = cellKey({ i: target.cell[0], j: target.cell[1] });
  const passability = passabilityFrom(space);
  if (!passability.cells.has(key) || passability.blocked.has(key)) {
    throw new Error(`the candidate feel ${context} cell is not walkable: ${target.space}/${key}`);
  }
}

function validateSpaces(spaces: Record<string, FeelSpace>, start: PortalTarget): void {
  validateLanding(spaces, start, "start");
  for (const [spaceName, space] of Object.entries(spaces)) {
    const cells = new Set(space.cells.map(cellKey));
    const { doorTiles } = wallAndDoorTiles(space.wall_runs, cells);
    for (const portal of space.portals) {
      const sourceKey = cellKey({ i: portal.cell[0], j: portal.cell[1] });
      if (!doorTiles.has(sourceKey)) {
        throw new Error(`the candidate feel portal is not a door tile: ${spaceName}/${sourceKey}`);
      }
      validateLanding(spaces, portal.to, "portal target");
    }
  }
}

export function parseFeelManifest(value: unknown): FeelManifest {
  if (isRecord(value) && value.schema_version === 1) {
    throw new Error("candidate feel manifest schema 1 is retired and refused");
  }
  if (!isRecord(value) || !hasExactKeys(value, ["schema_version", "assets", "start", "spaces"])) {
    throw new Error("the candidate feel manifest has unknown or missing top-level fields");
  }
  if (value.schema_version !== 2 || !isRecord(value.assets)) {
    throw new Error("the candidate feel manifest schema version or assets are invalid");
  }
  if (!hasExactKeys(value.assets, ASSET_GROUPS)) {
    throw new Error("the candidate feel asset groups are incomplete");
  }
  const assets: FeelManifest["assets"] = {
    terrain: parseAssetGroup(value.assets.terrain, "terrain"),
    walls: parseAssetGroup(value.assets.walls, "walls"),
    props: parseAssetGroup(value.assets.props, "props"),
    roofs: parseAssetGroup(value.assets.roofs, "roofs"),
  };
  requireAssets(assets, "terrain", REQUIRED_TERRAIN);
  requireAssets(assets, "walls", REQUIRED_WALLS);
  requireAssets(assets, "props", REQUIRED_PROPS);
  requireAssets(assets, "roofs", REQUIRED_ROOFS);

  const start = parsePortalTarget(value.start, "start");
  if (!isRecord(value.spaces) || Object.keys(value.spaces).length === 0) {
    throw new Error("the candidate feel manifest carries no spaces");
  }
  const spaces = Object.fromEntries(
    Object.entries(value.spaces).map(([name, space]) => {
      if (name.length === 0) throw new Error("a candidate feel space has no name");
      return [name, parseSpace(space, assets)];
    }),
  );
  validateSpaces(spaces, start);
  return { schema_version: 2, assets, start, spaces };
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
  const verified = async (key: string, file: AssetFile): Promise<void> => {
    const identity = `${file.file}:${file.sha256}`;
    let bytes = verifiedFiles.get(identity);
    if (bytes === undefined) {
      const response = await fetcher(`${prefix}${file.file}`, { cache: "no-store" });
      if (!response.ok) throw new Error(`${key}: asset file is missing`);
      bytes = await response.arrayBuffer();
      await verifySha256(bytes, file.sha256);
      verifiedFiles.set(identity, bytes);
    }
    assets.set(key, { bytes, file: file.file });
  };
  for (const group of ASSET_GROUPS) {
    for (const [name, row] of Object.entries(manifest.assets[group])) {
      await verified(`${group}/${name}`, row);
      if (row.normal !== null) await verified(`${group}/${name}/normal`, row.normal);
    }
  }
  return { manifest, assets };
}
