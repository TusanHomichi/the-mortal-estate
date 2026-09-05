import { expect, it } from "vitest";
import { frameTargets } from "../src/authoritative/targets";
import type { Frame } from "../src/authoritative/state";

const position = { realm: "test", level: "surface", position: { x: 4, y: 7 } };
function frame(): Frame {
  return { logical_time: "9007199254740993", ready_at: "9007199254740994", can_act: false,
    observer_actor_id: "player", observation_center: position,
    tiles: [{ position: position.position, terrain_id: "ground" }],
    actors: [{ actor_id: "player", position, name: "Player" }],
    corpses: [{ corpse_id: "body:1", location: position }],
    ground_items: [{ item_instance_id: "item:1", location: position }],
    gold_piles: [{ gold_pile_id: "gold:1", location: position }] };
}

it("addresses every observed kind in one identity space without moving its square", () => {
  const input = frame(), before = structuredClone(input);
  const { targets } = frameTargets(input, 768, 512);
  expect(targets.map(row => row.kind).sort()).toEqual(["actor", "corpse", "gold_pile", "ground_item", "tile"]);
  expect(targets.map(row => row.coordinate)).toEqual(Array(5).fill(position.position));
  expect(input).toEqual(before);
});

it("keeps dense occupant targets distinct and addressable", () => {
  const input = frame();
  input.actors = Array.from({ length: 128 }, (_, index) => ({ actor_id: `actor${index}`, position, name: "Actor" }));
  const { targets } = frameTargets(input, 768, 512);
  for (const target of targets.filter(row => row.kind !== "tile")) {
    const covered = targets.filter(row => row.kind !== "tile" && target.anchor.x >= row.hit_shape.x && target.anchor.y >= row.hit_shape.y
      && target.anchor.x < row.hit_shape.x + row.hit_shape.width && target.anchor.y < row.hit_shape.y + row.hit_shape.height);
    expect(covered).toEqual([target]);
  }
});

it("refuses absent squares, duplicate identities, and occupants outside the observed member", () => {
  const input = frame();
  input.tiles = [];
  expect(() => frameTargets(input, 768, 512)).toThrow();
  const duplicate = frame(); duplicate.actors.push(duplicate.actors[0]!);
  expect(() => frameTargets(duplicate, 768, 512)).toThrow("duplicate");
  const other = frame(); other.actors = [{ ...other.actors[0]!, position: { ...position, level: "other" } }];
  expect(() => frameTargets(other, 768, 512)).toThrow("outside");
});
