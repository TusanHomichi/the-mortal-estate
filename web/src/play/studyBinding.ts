import promotion from "../../../content/lands/first-expedition/promotion.json";
import studyReceipt from "./studyReceipt.json";
import { verifySha256 } from "../manifest";
import type { FeelManifest } from "../feelTypes";
import type { Snapshot } from "../authoritative/state";

/** The artwork can iterate independently of the accepted geographic encoding. */
export async function verifyStudyManifest(bytes: ArrayBuffer, receipt = studyReceipt): Promise<void> {
  if (receipt.schema_version !== 1 || receipt.status !== "candidate" ||
      receipt.geography_review_manifest_sha256 !== promotion.reviewed_encoding.review_manifest_sha256 ||
      !/^[0-9a-f]{64}$/.test(receipt.asset_manifest_sha256)) {
    throw new Error("The presentation receipt does not bind the accepted geography.");
  }
  await verifySha256(bytes, receipt.asset_manifest_sha256);
  const manifest = JSON.parse(new TextDecoder().decode(bytes)) as FeelManifest;
  for (const name of Object.values(receipt.actor_figures)) {
    if (!Object.hasOwn(manifest.figures, name)) throw new Error("The resident figure binding is unavailable.");
  }
}
interface StaticContext {
  site: { realm: string; level: string };
  visual_manifest_digest: string;
  bounds: { min: { x: number; y: number }; max: { x: number; y: number } };
  walkable_mask: { x: number; y: number }[];
}

/** Rust has decoded the context. This binds presentation to that authority;
 * the packet never supplies a move, eligibility, or transition destination. */
export function bindStudySpace(snapshot: Snapshot, manifest: FeelManifest): string {
  const context = snapshot.envelope.static_scene_context as StaticContext;
  const center = snapshot.envelope.frame.observation_center;
  if (context.visual_manifest_digest !== promotion.master.sha256 || center.realm !== "first_expedition" ||
      context.site.realm !== center.realm || context.site.level !== center.level) {
    throw new Error("This presentation study does not match the served geography.");
  }
  const space = manifest.spaces[center.level];
  if (!space || context.bounds.min.x < 0 || context.bounds.min.y < 0 ||
      context.bounds.max.x >= space.grid_extents.i || context.bounds.max.y >= space.grid_extents.j) {
    throw new Error("Presentation and server map bounds disagree.");
  }
  const expected = new Set(context.walkable_mask.map(p => `${p.x}:${p.y}`));
  const actual = space.cells.filter(c => c.walkable && c.i >= context.bounds.min.x && c.i <= context.bounds.max.x
    && c.j >= context.bounds.min.y && c.j <= context.bounds.max.y).map(c => `${c.i}:${c.j}`);
  if (actual.length !== expected.size || actual.some(key => !expected.has(key))) {
    throw new Error("Presentation and server floor masks disagree.");
  }
  return center.level;
}
