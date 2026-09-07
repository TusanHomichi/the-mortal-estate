import { expect, it } from "vitest";
import * as THREE from "three";
import { StudyActors } from "../src/play/studyActors";
import type { Frame } from "../src/authoritative/state";
import type { FeelSpace } from "../src/feelTypes";

it("keeps observed instances, moves their pick geometry and removes unseen actors", () => {
  const source = new THREE.Group();
  const geometry = new THREE.BoxGeometry(.6, 1.7, .4), material = new THREE.MeshStandardMaterial();
  const body = new THREE.Mesh(geometry, material); body.position.y = .85; source.add(body);
  const actors = new StudyActors();
  const space = { weather: false, grid_extents: { i: 5, j: 5 }, cells: [],
    structures: [{ file: "resident_provider.glb" }] } as unknown as FeelSpace;
  const frame = { observer_actor_id: "player", observation_center: { position: { x: 0, y: 0 } },
    actors: [{ actor_id: "provider", name: "Resident", position: { position: { x: 2, y: 2 } } }] } as Frame;
  const show = () => actors.present(frame, "room", space, new Map(), new Map([["structures/room/0", source]]), "unused", {});
  show(); const root = actors.group.children[0]; show(); expect(actors.group.children[0]).toBe(root);
  const ray = new THREE.Raycaster(new THREE.Vector3(2, 3, 2), new THREE.Vector3(0, -1, 0));
  expect(actors.pick(ray)).toBe("provider");
  expect(actors.pick(ray, .5)).toBeNull();
  frame.actors[0]!.position.position.x = 3; show(); actors.update(performance.now() + 500, .1);
  expect(actors.pick(ray)).toBeNull(); ray.ray.origin.x = 3;
  expect(actors.pick(ray)).toBe("provider");
  frame.actors = []; show(); expect(actors.pick(ray)).toBeNull();
  expect(actors.group.children).toHaveLength(0);
  actors.clear(); geometry.dispose(); material.dispose();
});
