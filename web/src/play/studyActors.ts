import * as THREE from "three";
import type { Frame } from "../authoritative/state";
import type { FeelSpace } from "../feelTypes";
import { createFigureInstance, type DecodedFigure, type FigureInstance } from "../space/figureRig";
import { createTerrainSurface } from "../terrainSurface";

interface Resident {
  root: THREE.Object3D;
  figure: FigureInstance | null;
  cell: { x: number; y: number };
  authoritativeCell: { x: number; y: number };
  motion: { from: { x: number; y: number }; started: number; walking: boolean } | null;
}

/** Persistent visual instances interpolate observed changes; they never invent movement. */
export class StudyActors {
  readonly group = new THREE.Group();
  private readonly residents = new Map<string, Resident>();

  clear(): void {
    for (const resident of this.residents.values()) resident.figure?.dispose();
    this.residents.clear(); this.group.clear();
  }

  present(frame: Frame, level: string, plan: FeelSpace, figures: Map<string, DecodedFigure>,
    structures: Map<string, THREE.Group>, defaultFigure: string, actorFigures: Readonly<Record<string, string>>): void {
    const visible = new Set<string>();
    const surface = createTerrainSurface(plan), at = frame.observation_center.position;
    for (const actor of frame.actors) {
      if (actor.actor_id === frame.observer_actor_id || (actor as typeof actor & { life_state: string }).life_state === "dead") continue;
      visible.add(actor.actor_id);
      const p = actor.position.position, sharing = p.x === at.x && p.y === at.y;
      const cell = { x: p.x + (sharing ? .23 : 0), y: p.y - (sharing ? .12 : 0) };
      let resident = this.residents.get(actor.actor_id);
      if (!resident) {
        const selected = actorFigures[actor.actor_id];
        const index = plan.structures.findIndex(row => row.file === `resident_${actor.actor_id}.glb`);
        let root: THREE.Object3D, figure: FigureInstance | null = null;
        if (!selected && index >= 0) {
          root = structures.get(`structures/${level}/${index}`)!.clone(true);
          root.position.set(cell.x, surface.heightAt(cell.x, cell.y), cell.y);
        } else {
          const source = figures.get(selected ?? defaultFigure);
          if (!source) throw new Error("The observed actor's figure is unavailable.");
          figure = createFigureInstance(source, { i: cell.x, j: cell.y }, { i: 0, j: 1 }, surface.heightAt);
          root = figure.root;
        }
        root.name = `ObservedActor_${actor.actor_id}`; root.userData.actorId = actor.actor_id;
        resident = { root, figure, cell, authoritativeCell: { ...p }, motion: null };
        this.residents.set(actor.actor_id, resident); this.group.add(root);
      } else if (resident.cell.x !== cell.x || resident.cell.y !== cell.y) {
        const dx = p.x - resident.authoritativeCell.x, dy = p.y - resident.authoritativeCell.y;
        const walking = dx !== 0 || dy !== 0;
        if (walking) resident.figure?.setFacing({ i: Math.sign(dx), j: Math.sign(dy) });
        resident.motion = { from: { x: resident.root.position.x, y: resident.root.position.z }, started: performance.now(), walking };
        resident.cell = cell;
        resident.authoritativeCell = { ...p };
      }
    }
    for (const [id, resident] of this.residents) if (!visible.has(id)) {
      resident.figure?.dispose(); this.group.remove(resident.root); this.residents.delete(id);
    }
  }

  update(now: number, delta: number): void {
    for (const resident of this.residents.values()) {
      if (resident.motion) {
        const t = Math.min(1, Math.max(0, (now - resident.motion.started) / 420));
        const x = THREE.MathUtils.lerp(resident.motion.from.x, resident.cell.x, t);
        const z = THREE.MathUtils.lerp(resident.motion.from.y, resident.cell.y, t);
        if (resident.figure) resident.figure.place(x, z);
        else { resident.root.position.x = x; resident.root.position.z = z; }
        resident.figure?.setGait(t < 1 && resident.motion.walking ? "walk" : "idle");
        if (t === 1) resident.motion = null;
      }
      resident.figure?.update(delta);
    }
  }

  pick(ray: THREE.Raycaster, blockingDistance = Infinity): string | null {
    this.group.updateMatrixWorld(true);
    for (const hit of ray.intersectObject(this.group, true)) {
      if (hit.distance > blockingDistance) return null;
      let object: THREE.Object3D | null = hit.object;
      while (object && object !== this.group) {
        if (typeof object.userData.actorId === "string") return object.userData.actorId;
        object = object.parent;
      }
    }
    return null;
  }
}
