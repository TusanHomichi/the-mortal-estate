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

export interface AssetFile {
  file: string;
  sha256: string;
}

/**
 * A prop card may carry a normal sheet beside its colour sheet, and may declare
 * itself `flat` — a thing with no side, a rug — which is the only card a
 * placement may lay on the floor. Nothing else may carry either.
 */
export interface AssetRow extends AssetFile {
  normal: AssetFile | null;
  flat: boolean;
}

export type AssetRows = Record<string, AssetRow>;

/** One skinned outfit part on the figure's skeleton, with the files its glTF names. */
export interface FigurePart extends AssetFile {
  sidecars: AssetFile[];
}

/**
 * A live figure (owner ruling, 2026-09-03): a rigged glTF, the files it names,
 * a clip library, skinned parts on the same skeleton, and the material's inputs —
 * the figure's own treated-card palette and a rim darkening. Every file is
 * digest-bound; the client resolves the names a glTF asks for only against
 * these, never the network.
 */
export interface FigureRow {
  rig: AssetFile;
  sidecars: AssetFile[];
  clips: AssetFile;
  parts: FigurePart[];
  palette: [number, number, number][];
  rim: number;
  idle: string;
  /** The clips for the walk between pulses, by the route's pace. */
  gait: { walk: string; run: string; sprint: string };
}

export type FigureRows = Record<string, FigureRow>;

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

/**
 * A card placement. `card_height` is the world height the card's image spans —
 * feet at `elevation`, top at `elevation + card_height` — not the subject's own
 * height: a low, long thing rendered at the ruled angle is mostly its depth,
 * and sizing it by its own height made beds and tables read doll-sized (owner
 * ruling, 2026-09-03). `floor` lays the card flat on the ground, its up toward
 * north, for things that are genuinely flat — a rug — and nothing with a side.
 */
export interface PropPlacement {
  kind: string;
  cell_anchor: [number, number];
  elevation: number;
  card_height: number;
  sway: boolean;
  mirror: boolean;
  facing: "view" | "+z" | "+x" | "floor";
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
  schema_version: 5;
  assets: Record<AssetGroup, AssetRows>;
  figures: FigureRows;
  /** Which figure the start places; the client carries no caretaker of its own. */
  caretaker: { figure: string };
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
