export const ASSET_GROUPS = ["terrain", "walls", "props"] as const;
export type AssetGroup = (typeof ASSET_GROUPS)[number];

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
  "lantern_post",
  "shrine_table",
  "grave_marker",
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
  nominal_height: number;
  sway: boolean;
  mirror: boolean;
}

export interface FeelLayout {
  grid_extents: { i: number; j: number };
  cells: CellPlan[];
  wall_runs: WallRun[];
  props: PropPlacement[];
  light_sources: {
    lantern_glass: [number, number, number];
    candles: [number, number, number][];
  };
}

export interface FeelManifest {
  schema_version: 1;
  assets: Record<AssetGroup, AssetRows>;
  layout: FeelLayout;
}

export interface VerifiedAsset {
  bytes: ArrayBuffer;
  file: string;
}

export interface VerifiedAssetPacket {
  manifest: FeelManifest;
  assets: Map<string, VerifiedAsset>;
}
