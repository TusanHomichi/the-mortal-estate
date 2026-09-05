import type { Coord, Frame } from "./state";

export interface Target {
  index: number; identity: string; source_identity: string; kind: string;
  coordinate: Coord; presentation_layer: "squares" | "occupants";
  anchor: Coord;
  hit_shape: { kind: "rect"; x: number; y: number; width: number; height: number };
}

/** One frame-row identity owner for drawing, pointer facts, and capture. */
export function frameTargets(frame: Frame, width: number, height: number) {
  if (!frame.tiles.length) throw new Error("the frame has no visible squares to capture");
  const xs = frame.tiles.map(row => row.position.x), ys = frame.tiles.map(row => row.position.y);
  const minX = Math.min(...xs), minY = Math.min(...ys), maxX = Math.max(...xs), maxY = Math.max(...ys);
  const pitch = Math.floor(Math.min((width - 48) / (maxX - minX + 1), (height - 48) / (maxY - minY + 1)));
  if (pitch < 12) throw new Error("capture viewport cannot address the visible frame");
  const origin = { x: Math.floor((width - (maxX - minX + 1) * pitch) / 2), y: Math.floor((height - (maxY - minY + 1) * pitch) / 2) };
  const targets: Target[] = [];
  const visible = new Set(frame.tiles.map(row => `${row.position.x}:${row.position.y}`));
  const add = (kind: string, id: string, coordinate: Coord, slot: number, columns = 3) => {
    if (!visible.has(`${coordinate.x}:${coordinate.y}`)) throw new Error("occupant has no visible square");
    const x = origin.x + (coordinate.x - minX) * pitch;
    const y = origin.y + (coordinate.y - minY) * pitch;
    const size = kind === "tile" ? pitch : Math.floor(pitch / columns);
    const hit = kind === "tile" ? { x, y, width: size, height: size }
      : { x: x + 1 + (slot % columns) * size, y: y + 1 + Math.floor(slot / columns) * size, width: size - 1, height: size - 1 };
    targets.push({ index: targets.length + 1, identity: `${kind}:${id}`, source_identity: id,
      kind, coordinate: { ...coordinate }, presentation_layer: kind === "tile" ? "squares" : "occupants",
      anchor: { x: hit.x + Math.floor(hit.width / 2), y: hit.y + Math.floor(hit.height / 2) },
      hit_shape: { kind: "rect", ...hit } });
  };
  for (const row of [...frame.tiles].sort((a, b) => a.position.y - b.position.y || a.position.x - b.position.x)) {
    add("tile", `${row.position.x}:${row.position.y}`, row.position, 0);
  }
  const occupants = [
    ...frame.actors.map(row => ({ kind: "actor", id: row.actor_id, location: row.position })),
    ...frame.corpses.map(row => ({ kind: "corpse", id: row.corpse_id, location: row.location })),
    ...frame.ground_items.map(row => ({ kind: "ground_item", id: row.item_instance_id, location: row.location })),
    ...frame.gold_piles.map(row => ({ kind: "gold_pile", id: row.gold_pile_id, location: row.location })),
  ].sort((a, b) => a.kind.localeCompare(b.kind) || a.id.localeCompare(b.id));
  const totals = new Map<string, number>();
  for (const row of occupants) {
    const coord = row.location.position, key = `${coord.x}:${coord.y}`;
    totals.set(key, (totals.get(key) ?? 0) + 1);
  }
  const counts = new Map<string, number>();
  for (const row of occupants) {
    if (row.location.realm !== frame.observation_center.realm || row.location.level !== frame.observation_center.level) {
      throw new Error("occupant is outside the observed member");
    }
    const coord = row.location.position, key = `${coord.x}:${coord.y}`, count = counts.get(key) ?? 0;
    const columns = Math.max(3, Math.ceil(Math.sqrt(totals.get(key)!)));
    if (Math.floor(pitch / columns) < 2) throw new Error("viewport cannot address all observed occupants");
    counts.set(key, count + 1);
    add(row.kind, row.id, coord, count, columns);
  }
  if (new Set(targets.map(row => row.identity)).size !== targets.length) throw new Error("duplicate target identity");
  return { targets, camera: { kind: "orthographic_square_lattice", square_pitch_px: pitch,
    square_origin_px: origin, view_origin_px: { x: 0, y: 0 }, view_size_px: { x: width, y: height },
    square_bounds: { min_x: minX, min_y: minY, max_x: maxX, max_y: maxY, columns: maxX - minX + 1, rows: maxY - minY + 1 } } };
}
