export const ASSET_GROUPS = ["terrain", "walls", "props", "roofs"] as const;
export type AssetGroup = (typeof ASSET_GROUPS)[number];

export const REQUIRED_TERRAIN = ["grass", "stone", "earth", "floor_planks"] as const;

export const REQUIRED_WALLS = [
  "plinth",
  "plaster",
  "cap_front",
  "cap_top",
  "sill",
  "post",
  "door",
] as const;

export const REQUIRED_PROPS = [
  "caretaker",
  "tree",
  "tree_slim",
  "tree_broad",
  "tree_bare",
  "lantern_post",
  "shrine_table",
  "grave_marker",
  "fire",
] as const;

export const REQUIRED_ROOFS = [
  "shingle_slope",
  "shingle_ridge",
  "shingle_eave",
] as const;

export interface AssetRow {
  file: string;
  sha256: string;
}

export type AssetRows = Record<string, AssetRow>;

export interface CellPlan {
  i: number;
  j: number;
  material: string;
}

export type WallAxis = "x" | "z";

export interface WallRun {
  axis: WallAxis;
  start: [number, number];
  cells: number;
  door_interval: [number, number] | null;
}

export interface PropPlacement {
  kind: string;
  cell_anchor: [number, number];
  elevation: number;
  nominal_height: number;
  sway: boolean;
  mirror: boolean;
  facing: "view" | "+z" | "+x";
}

export interface HearthFixturePlacement {
  kind: "hearth";
  cell: [number, number];
  against: "north" | "west";
}

export type FixturePlacement = HearthFixturePlacement;

export interface RoofPlacement {
  footprint: { i0: number; j0: number; i1: number; j1: number };
  ridge_axis: WallAxis;
  eave_height: number;
  ridge_height: number;
  material: string;
}

export interface PortalTarget {
  space: string;
  cell: [number, number];
}

export interface PortalPlacement {
  cell: [number, number];
  to: PortalTarget;
}

export interface FeelSpace {
  grid_extents: { i: number; j: number };
  cells: CellPlan[];
  wall_runs: WallRun[];
  roofs: RoofPlacement[];
  props: PropPlacement[];
  fixtures: FixturePlacement[];
  light_sources: {
    lantern_glass: [number, number, number] | null;
    candles: [number, number, number][];
  };
  weather: boolean;
  portals: PortalPlacement[];
}

export interface FeelManifest {
  schema_version: 2;
  assets: Record<AssetGroup, AssetRows>;
  start: PortalTarget;
  spaces: Record<string, FeelSpace>;
}

export interface VerifiedAsset {
  bytes: ArrayBuffer;
  file: string;
}

export interface VerifiedAssetPacket {
  manifest: FeelManifest;
  assets: Map<string, VerifiedAsset>;
}
