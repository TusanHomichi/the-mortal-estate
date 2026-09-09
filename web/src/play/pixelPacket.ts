import { validatePixelEffects, pixelEffectFiles, type PixelEffectsConfig } from "./pixelEffectsConfig";
import promotion from "../../../content/lands/first-expedition/promotion.json";
import geography from "../../../content/lands/first-expedition/generated/workbench_projection.json";
import receipt from "./pixelReceipt.json";
import { verifySha256 } from "../assetDigest";
import type { Snapshot, Coord } from "../authoritative/state";
import type { PixelProjection } from "./pixelGeometry";

export interface PixelFile { file: string; sha256: string }
export interface PixelSprite extends PixelFile { body_height: number; foot: Coord }
export interface PixelFigure {
  rotations: Record<string, PixelSprite>;
  walk: Record<string, PixelSprite[]>;
  size: number;
}
export interface PixelForeground extends PixelFile { id: string; x: number; y: number; depth: number }
export interface PixelExterior {
  background: PixelFile; width: number; height: number; projection: PixelProjection;
  foreground: PixelForeground[];
  entrances: { transition: string; bounds: { x: number; y: number; width: number; height: number } }[];
}
export interface PixelRoomPatch { x:number; y:number; width:number; height:number; depth:number }
export interface PixelManifest {
  schema_version: number;
  status: string;
  geography_review_manifest_sha256: string;
  geography_master_sha256: string;
  background: PixelFile;
  temple_foreground: PixelRoomPatch[];
  exterior: PixelExterior;
  effects: PixelEffectsConfig;
  figures: Record<string, PixelFigure>;
  actor_figures: Record<string, string>;
}
export interface PixelPacket { manifest: PixelManifest; images: Map<string, HTMLImageElement> }

export function validatePixelManifest(value: PixelManifest): void {
  if (value.schema_version !== 4 || value.status !== "candidate" ||
      value.geography_review_manifest_sha256 !== promotion.reviewed_encoding.review_manifest_sha256 ||
      value.geography_master_sha256 !== promotion.master.sha256 ||
      !value.figures.traveler || !value.figures.tomas || !value.figures.maude ||
      value.actor_figures.tomas !== "tomas" || value.actor_figures.balm_seller !== "maude") {
    throw new Error("Pixel packet does not bind the selected temple artwork.");
  }
  validateExterior(value.exterior);
  validatePixelEffects(value.effects);
  if(!Array.isArray(value.temple_foreground)||!value.temple_foreground.length||value.temple_foreground.length>32)throw new Error("Missing temple foreground.");
  for(const p of value.temple_foreground)if(![p.x,p.y,p.width,p.height,p.depth].every(Number.isInteger)||
    p.x<0||p.y<0||p.width<1||p.height<1||p.x+p.width>512||p.y+p.height>512||p.depth<p.y||p.depth>512)throw new Error("Invalid temple foreground calibration.");
  for (const figure of Object.values(value.figures)) {
    if (!Number.isInteger(figure.size) || figure.size < 16 || figure.size > 512) {
      throw new Error("Invalid pixel sprite calibration.");
    }
    for (const frame of [...Object.values(figure.rotations),...Object.values(figure.walk).flat()]) {
      if (!Number.isInteger(frame.body_height) || frame.body_height<16 || frame.body_height>figure.size ||
          !frame.foot || !Number.isFinite(frame.foot.x) || !Number.isFinite(frame.foot.y) ||
          frame.foot.x<0 || frame.foot.x>figure.size || frame.foot.y<frame.body_height || frame.foot.y>figure.size) {
        throw new Error("Invalid pixel frame anchor.");
      }
    }
    for (const direction of ["north", "east", "south", "west"]) {
      if (!figure.rotations[direction] || !figure.walk[direction]?.length || figure.walk[direction]!.length > 16) {
        throw new Error("Incomplete directional pixel sprite.");
      }
    }
    const diagonals=["north-east","south-east","south-west","north-west"];
    if (diagonals.some(direction=>figure.walk[direction]) && diagonals.some(direction=>
      !figure.rotations[direction] || !figure.walk[direction]?.length || figure.walk[direction]!.length>16)) {
      throw new Error("Incomplete diagonal pixel gait.");
    }
  }
  for (const selected of Object.values(value.actor_figures)) if (!value.figures[selected]) throw new Error("Missing pixel figure.");
  for (const file of pixelFiles(value)) {
    if (!/^[a-zA-Z0-9_-]+\.png$/.test(file.file) || !/^[a-f0-9]{64}$/.test(file.sha256)) throw new Error("Invalid pixel asset reference.");
  }
  const digests = new Map<string, string>();
  for (const file of pixelFiles(value)) {
    if (digests.has(file.file) && digests.get(file.file) !== file.sha256) throw new Error("Conflicting pixel asset digests.");
    digests.set(file.file, file.sha256);
  }
}
export function pixelFiles(value: PixelManifest): PixelFile[] {
  return [...pixelEffectFiles(value.effects), value.background, value.exterior.background, ...value.exterior.foreground, ...Object.values(value.figures).flatMap(figure =>
    [...Object.values(figure.rotations), ...Object.values(figure.walk).flat()])];
}
export async function loadPixelPacket(): Promise<PixelPacket> {
  const read = async (file: string) => {
    const response = await fetch(`/feel-assets/${file}`, { credentials: "omit", cache: "no-store" });
    if (!response.ok) throw new Error("Pixel artwork unavailable.");
    return response.arrayBuffer();
  };
  const bytes = await read("pixel-manifest.json");
  await verifySha256(bytes, receipt.manifest_sha256);
  const manifest = JSON.parse(new TextDecoder().decode(bytes)) as PixelManifest;
  validatePixelManifest(manifest);
  const images = new Map<string, HTMLImageElement>();
  await Promise.all([...new Map(pixelFiles(manifest).map(file => [file.file, file])).values()].map(async file => {
    const data = await read(file.file); await verifySha256(data, file.sha256);
    const url = URL.createObjectURL(new Blob([data], { type: "image/png" }));
    try {
      const image = new Image(); image.src = url; await image.decode();
      if (image.naturalWidth > 4096 || image.naturalHeight > 4096) throw new Error("Oversized pixel asset.");
      images.set(file.file, image);
    } finally { URL.revokeObjectURL(url); }
  }));
  if (images.get(manifest.background.file)!.naturalWidth !== 512 || images.get(manifest.background.file)!.naturalHeight !== 512 ||
      images.get(manifest.exterior.background.file)!.naturalWidth !== manifest.exterior.width ||
      images.get(manifest.exterior.background.file)!.naturalHeight !== manifest.exterior.height) {
    throw new Error("Pixel scene dimensions disagree with their calibration.");
  }
  for (const asset of manifest.exterior.foreground) {
    const image=images.get(asset.file)!;
    if (asset.x+image.naturalWidth>manifest.exterior.width || asset.y+image.naturalHeight>manifest.exterior.height) {
      throw new Error("Pixel foreground exceeds its scene.");
    }
  }
  for(const [name,scene] of Object.entries(manifest.effects.scenes)) {
    const size=name==="temple" ? {width:512,height:512} : manifest.exterior;
    for(const file of [scene.material,scene.normals]) {
      const image=images.get(file.file)!;
      if(image.naturalWidth!==size.width||image.naturalHeight!==size.height)throw new Error("Pixel effect map dimensions disagree.");
    }
  }
  for(const p of Object.values(manifest.effects.profiles)) {
    const image=images.get(p.grade.file)!;
    if(image.naturalWidth!==256||image.naturalHeight!==16)throw new Error("Invalid pixel colour lookup table.");
  }
  return { manifest, images };
}

function validateExterior(scene: PixelExterior): void {
  const positive=(n:number)=>Number.isInteger(n)&&n>0&&n<=4096;
  if (!scene?.background || !positive(scene.width) || !positive(scene.height) ||
      !scene.projection?.origin || !scene.projection.step ||
      ![scene.projection.origin.x,scene.projection.origin.y].every(Number.isInteger) ||
      ![scene.projection.step.x,scene.projection.step.y].every(positive) ||
      !Array.isArray(scene.foreground) || !scene.foreground.length || !Array.isArray(scene.entrances)) {
    throw new Error("Invalid connected exterior calibration.");
  }
  const ids=new Set<string>();
  for (const layer of scene.foreground) {
    if (!layer.id || ids.has(layer.id) || ![layer.x,layer.y,layer.depth].every(Number.isInteger) ||
        layer.x<0 || layer.y<0 || layer.x>=scene.width || layer.y>=scene.height || layer.depth<layer.y || layer.depth>scene.height) {
      throw new Error("Invalid exterior foreground calibration.");
    }
    ids.add(layer.id);
  }
  const arrival=geography.members.find(member=>member.member==="arrival")!;
  const transitions=new Map(arrival.transitions.map(edge=>[edge.id,edge]));
  if (scene.entrances.length!==transitions.size) throw new Error("Incomplete town entrance calibration.");
  for (const entrance of scene.entrances) {
    const edge=transitions.get(entrance.transition),b=entrance.bounds;
    if (!edge || !b || ![b.x,b.y,b.width,b.height].every(Number.isInteger) || b.x<0 || b.y<0 ||
        !positive(b.width) || !positive(b.height) || b.width>scene.projection.step.x*2 || b.height>scene.projection.step.y*2 ||
        b.x+b.width>scene.width || b.y+b.height>scene.height) throw new Error("Invalid town entrance calibration.");
    const x=scene.projection.origin.x+edge.access.x*scene.projection.step.x;
    const y=scene.projection.origin.y+edge.access.y*scene.projection.step.y;
    if (x<b.x || x>=b.x+b.width || y<b.y || y>=b.y+b.height) throw new Error("Town doorway and authored entrance disagree.");
    transitions.delete(entrance.transition);
  }
}

/** Match static context to its authored member; never derive an action from art. */
export function bindPixelSpace(snapshot: Snapshot): string {
  const center = snapshot.envelope.frame.observation_center;
  const context = snapshot.envelope.static_scene_context as {
    visual_manifest_digest: string; site: { realm: string; level: string };
    bounds: { min: Coord; max: Coord }; walkable_mask: Coord[];
  };
  const member = geography.members.find(member => member.member === center.level);
  if (center.realm !== "first_expedition" || context.site.realm !== center.realm || context.site.level !== center.level ||
      context.visual_manifest_digest !== promotion.master.sha256 || !member ||
      context.bounds.min.x < 0 || context.bounds.min.y < 0 ||
      context.bounds.max.x >= member.width || context.bounds.max.y >= member.height ||
      context.bounds.min.x > context.bounds.max.x || context.bounds.min.y > context.bounds.max.y) {
    throw new Error("Pixel artwork and served geography disagree.");
  }
  // Outdoors the server sends a bounded window, not the entire authored member.
  const expected = new Set(member.cells.filter(cell => cell.passable &&
    cell.x >= context.bounds.min.x && cell.x <= context.bounds.max.x &&
    cell.y >= context.bounds.min.y && cell.y <= context.bounds.max.y).map(cell => `${cell.x}:${cell.y}`));
  if (context.walkable_mask.length !== expected.size ||
      new Set(context.walkable_mask.map(cell => `${cell.x}:${cell.y}`)).size !== expected.size ||
      context.walkable_mask.some(cell => !expected.has(`${cell.x}:${cell.y}`))) throw new Error("Pixel presentation floor mask disagrees.");
  return center.level;
}
