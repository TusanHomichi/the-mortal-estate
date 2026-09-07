import * as THREE from "three";
import { footprintsFromPath } from "../walk/footprints";
import { makeSoleTexture } from "../walk/soleTexture";
import type { WalkPresentation } from "./pathControls";

/** Shared original footprint artwork placed on the rendered terrain surface. */
export class PathOverlay {
  readonly group = new THREE.Group();
  private identity = "";
  private textures = { draft: makeSoleTexture("draft"), committed: makeSoleTexture("committed") };
  clear(): void {
    for (const child of this.group.children) {
      const mesh = child as THREE.Mesh<THREE.BufferGeometry, THREE.Material>;
      mesh.geometry.dispose(); mesh.material.dispose();
    }
    this.group.clear(); this.identity = "";
  }
  present(view: WalkPresentation, height: (x: number,z: number) => number): void {
    const identity = JSON.stringify(view);
    if (identity === this.identity) return;
    this.clear(); this.identity = identity;
    for (const print of footprintsFromPath(view.route ?? [])) {
      const material = new THREE.MeshBasicMaterial({ map: this.textures[view.kind], transparent:true,
        opacity:view.kind === "draft" ? .7 : 1, blending:THREE.AdditiveBlending, depthWrite:false,
        depthTest:true, toneMapped:false });
      const mesh = new THREE.Mesh(new THREE.PlaneGeometry(.18,.27),material);
      mesh.rotation.x = -Math.PI/2; mesh.rotation.z = print.angle;
      mesh.scale.x = print.foot === "left" ? -1 : 1;
      mesh.position.set(print.position.x,height(print.position.x,print.position.z)+.025,print.position.z);
      mesh.renderOrder = 20; this.group.add(mesh);
    }
    if (view.hover) {
      const { x,y } = view.hover;
      const points = [[-.48,-.48],[.48,-.48],[.48,.48],[-.48,.48]].map(([dx,dz]) =>
        new THREE.Vector3(x+dx!,height(x+dx!,y+dz!)+.025,y+dz!));
      const line = new THREE.LineLoop(new THREE.BufferGeometry().setFromPoints(points),new THREE.LineBasicMaterial({
        color:view.cursor === "waiting" ? 0xf2c66d : view.cursor === "refused" ? 0xf18d85 : 0xa6b8c9,
        transparent:true,opacity:.75,depthTest:false,depthWrite:false,toneMapped:false }));
      line.renderOrder=20; this.group.add(line);
    }
  }
  dispose(): void { this.clear(); this.textures.draft.dispose(); this.textures.committed.dispose(); }
}
