import * as THREE from "three";
import type { Snapshot } from "./state";
import { frameTargets, type Target } from "./targets";

const colors: Record<string, number> = { tile: 0x43534c, actor: 0xe6bd75, corpse: 0xb77c82, ground_item: 0x85b6c5, gold_pile: 0xe7d570 };

/** A read-only diagnostic view of actual observer rows. No packet or rules. */
export class AuthoritativeRenderer {
  readonly renderer: THREE.WebGLRenderer;
  readonly camera: THREE.OrthographicCamera;
  private scene = new THREE.Scene();
  private readonly raycaster = new THREE.Raycaster();
  targets: Target[] = [];
  cameraIdentity: ReturnType<typeof frameTargets>["camera"] | null = null;
  snapshot: Snapshot | null = null;

  constructor(readonly canvas: HTMLCanvasElement, readonly width: number, readonly height: number) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: false, preserveDrawingBuffer: true });
    this.renderer.setPixelRatio(1);
    this.renderer.setSize(width, height);
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.setClearColor(0x151d20);
    this.camera = new THREE.OrthographicCamera(0, width, height, 0, 0.1, 100);
    this.camera.position.z = 10;
  }

  clear(): void {
    for (const object of this.scene.children) {
      const mesh = object as THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
      mesh.geometry.dispose(); mesh.material.dispose();
    }
    this.scene.clear(); this.targets = []; this.snapshot = null; this.cameraIdentity = null;
    this.renderer.render(this.scene, this.camera);
  }

  present(snapshot: Snapshot): void {
    const projected = frameTargets(snapshot.envelope.frame, this.width, this.height);
    this.clear();
    this.targets = projected.targets; this.cameraIdentity = projected.camera; this.snapshot = snapshot;
    for (const target of this.targets) {
      const hit = target.hit_shape;
      const material = new THREE.MeshBasicMaterial({ color: colors[target.kind]!, toneMapped: false });
      if (target.kind === "tile") {
        const tile = snapshot.envelope.frame.tiles.find(row => row.position.x === target.coordinate.x && row.position.y === target.coordinate.y)!;
        let hash = 0;
        for (const char of tile.terrain_id ?? "unobserved") hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
        // Diagnostic terrain hues and a cell checker carry no gameplay meaning.
        material.color.setHSL((hash % 360) / 360, .15,
          .23 + ((target.coordinate.x + target.coordinate.y) % 2) * .035);
      }
      const mesh = new THREE.Mesh(new THREE.PlaneGeometry(hit.width, hit.height), material);
      mesh.position.set(hit.x + hit.width / 2, this.height - hit.y - hit.height / 2, target.presentation_layer === "squares" ? 0 : 1);
      mesh.userData.target = target;
      this.scene.add(mesh);
    }
    this.renderer.render(this.scene, this.camera);
  }

  pointer(x: number, y: number): Target | null {
    if (x < 0 || y < 0 || x >= this.width || y >= this.height) return null;
    this.scene.updateMatrixWorld(true); this.camera.updateMatrixWorld(true);
    this.raycaster.setFromCamera(new THREE.Vector2(x / this.width * 2 - 1, 1 - y / this.height * 2), this.camera);
    return this.raycaster.intersectObjects(this.scene.children, false)[0]?.object.userData.target ?? null;
  }

  identityRaster(): Uint8Array<ArrayBuffer> {
    if (!this.snapshot) throw new Error("no authoritative frame is presented");
    const saved = this.scene.children.map(object => (object as THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>).material.color.clone());
    const background = this.renderer.getClearColor(new THREE.Color());
    const colorSpace = this.renderer.outputColorSpace;
    try {
      this.renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
      this.renderer.setClearColor(0);
      this.scene.children.forEach((object, index) => {
        const id = this.targets[index]!.index;
        (object as THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>).material.color.setRGB((id >> 8) / 255, (id & 255) / 255, 0, THREE.LinearSRGBColorSpace);
      });
      this.renderer.render(this.scene, this.camera);
      const gl = this.renderer.getContext(), pixels = new Uint8Array(this.width * this.height * 4);
      gl.readPixels(0, 0, this.width, this.height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      const header = new TextEncoder().encode(`P5\n${this.width} ${this.height}\n65535\n`);
      const raster = new Uint8Array(header.length + this.width * this.height * 2);
      raster.set(header);
      for (let y = 0; y < this.height; y++) for (let x = 0; x < this.width; x++) {
        const source = ((this.height - 1 - y) * this.width + x) * 4;
        const target = header.length + (y * this.width + x) * 2;
        raster[target] = pixels[source]!; raster[target + 1] = pixels[source + 1]!;
      }
      return raster;
    } finally {
      this.scene.children.forEach((object, index) => (object as THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>).material.color.copy(saved[index]!));
      this.renderer.outputColorSpace = colorSpace;
      this.renderer.setClearColor(background); this.renderer.render(this.scene, this.camera);
    }
  }

  dispose(): void { this.clear(); this.renderer.dispose(); }
}
