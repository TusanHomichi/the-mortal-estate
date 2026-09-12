import promotion from "../../../content/lands/first-expedition/promotion.json";
import geography from "../../../content/lands/first-expedition/generated/workbench_projection.json";
import type { Snapshot, Coord } from "../authoritative/state";
export { geography };

/** Match static context to its authored member; never derive an action from art. */
export function bindWorldSpace(snapshot: Snapshot): string {
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
    throw new Error("World artwork and served geography disagree.");
  }
  // Outdoors the server sends a bounded window, not the entire authored member.
  const expected = new Set(member.cells.filter(cell => cell.passable &&
    cell.x >= context.bounds.min.x && cell.x <= context.bounds.max.x &&
    cell.y >= context.bounds.min.y && cell.y <= context.bounds.max.y).map(cell => `${cell.x}:${cell.y}`));
  if (context.walkable_mask.length !== expected.size ||
      new Set(context.walkable_mask.map(cell => `${cell.x}:${cell.y}`)).size !== expected.size ||
      context.walkable_mask.some(cell => !expected.has(`${cell.x}:${cell.y}`))) throw new Error("World presentation floor mask disagrees.");
  return center.level;
}
