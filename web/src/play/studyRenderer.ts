import * as THREE from "three";
import type { Snapshot, Coord } from "../authoritative/state";
import { cameraFocusFor, createFeelCamera, focusFeelCamera } from "../camera";
import { createTerrainSurface } from "../terrainSurface";
import { fetchVerifiedAssetPacket } from "../manifest";
import type { VerifiedAssetPacket } from "../feelTypes";
import { SpaceScene } from "../space/SpaceScene";
import { decodeTextures, type DecodedTexture } from "../space/textures";
import { decodeStructures } from "../space/structures";
import { decodeFigures, disposeDecodedFigures, disposeFigureSources, type DecodedFigure } from "../space/figureRig";
import { StudyActors } from "./studyActors";
import studyReceipt from "./studyReceipt.json";
import { bindStudySpace, verifyStudyManifest } from "./studyBinding";
import { PathOverlay } from "./pathOverlay";
import type { WalkPresentation } from "./pathControls";
import type { Cell } from "../walk/layoutPassability";

/** Private presentation study over real server frames. No local walk presenter. */
export class ExpeditionStudyRenderer {
  readonly renderer: THREE.WebGLRenderer;
  readonly camera: THREE.OrthographicCamera;
  private readonly scene = new THREE.Scene();
  private readonly wind = new Map<string, THREE.DataTexture>();
  private active: SpaceScene | null = null;
  private level: string | null = null;
  private snapshot: Snapshot | null = null;
  private readonly occupants = new THREE.Group();
  private readonly residents = new StudyActors();
  private readonly pathOverlay = new PathOverlay();
  private walk: WalkPresentation | null = null;
  private readonly looseGeometry = new THREE.SphereGeometry(.13, 8, 6);
  private readonly looseMaterial = new THREE.MeshStandardMaterial({ color: 0xcba36c, roughness: .8 });
  private animation = 0;
  private elapsed = 0;
  private motion: { route: Cell[]; started: number } | null = null;

  private constructor(readonly canvas: HTMLCanvasElement, readonly width: number, readonly height: number,
    private readonly packet: VerifiedAssetPacket, private readonly textures: Map<string, DecodedTexture>,
    private readonly figures: Map<string, DecodedFigure>, private readonly structures: Map<string, THREE.Group>) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.renderer.shadowMap.enabled = true;
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    this.renderer.setSize(width, height, false);
    this.camera = createFeelCamera(width, height, { i: 9, j: 31 }, 0);
    this.scene.add(this.occupants, this.residents.group, this.pathOverlay.group);
    const animate = (now: number) => {
      const seconds = now / 1000, delta = Math.min(.1, Math.max(0, seconds - this.elapsed));
      this.elapsed = seconds;
      if (this.active && this.motion) {
        const { route } = this.motion, steps = route.length - 1;
        const fraction = Math.min(1, Math.max(0, (now - this.motion.started) / (420 * steps)));
        const segment = Math.min(steps-1,Math.floor(fraction*steps)), along = fraction*steps-segment;
        const from = route[segment]!, to = route[segment+1]!;
        this.active.caretaker.place(from.i+(to.i-from.i)*along,from.j+(to.j-from.j)*along);
        this.active.caretaker.setFacing({ i:Math.sign(to.i-from.i),j:Math.sign(to.j-from.j) });
        this.active.caretaker.setGait(fraction < 1 ? steps === 3 ? "sprint" : steps === 2 ? "run" : "walk" : "idle");
        if (fraction === 1) this.motion = null;
      }
      this.active?.update(seconds);
      this.active?.updateOcclusion(now);
      this.residents.update(now, delta);
      this.renderer.render(this.scene, this.camera);
      this.animation = requestAnimationFrame(animate);
    };
    this.animation = requestAnimationFrame(animate);
    canvas.dataset.presentation = "first-expedition-study";
  }

  static async create(canvas: HTMLCanvasElement, width: number, height: number): Promise<ExpeditionStudyRenderer> {
    const packet = await fetchVerifiedAssetPacket("/feel-assets/", async (input, init) => {
      const response = await fetch(input, init);
      if (String(input).endsWith("feel-manifest.json") && response.ok) {
        const bytes = await response.arrayBuffer();
        await verifyStudyManifest(bytes);
        return new Response(bytes, { headers: response.headers, status: response.status });
      }
      return response;
    });
    const textures = await decodeTextures(packet);
    let figures: Map<string, DecodedFigure> | null = null;
    let structures: Map<string, THREE.Group> | null = null;
    try {
      figures = await decodeFigures(packet);
      structures = await decodeStructures(packet);
      return new ExpeditionStudyRenderer(canvas, width, height, packet, textures, figures, structures);
    } catch (error) {
      for (const value of textures.values()) value.texture.dispose();
      if (figures) disposeDecodedFigures(figures);
      if (structures) disposeFigureSources([...new Set(structures.values())]);
      throw error;
    }
  }

  private clearOccupants(): void {
    this.residents.clear();
    this.occupants.clear();
  }

  clear(): void {
    this.snapshot = null; this.motion = null;
    this.walk = null; this.pathOverlay.clear();
    this.active?.dispose(); this.active = null; this.level = null;
    this.clearOccupants();
    this.scene.background = new THREE.Color(0x151d20);
    this.renderer.render(this.scene, this.camera);
    delete this.canvas.dataset.studyLevel;
  }

  present(snapshot: Snapshot): void {
    let level: string;
    try { level = bindStudySpace(snapshot, this.packet.manifest); }
    catch (error) { this.clear(); throw error; }
    const frame = snapshot.envelope.frame, at = frame.observation_center.position;
    const plan = this.packet.manifest.spaces[level]!;
    const focus = cameraFocusFor(plan, { i: at.x, j: at.y });
    if (this.level !== level) {
      this.clear();
      // Candidate residents are removed from scenery. Only observed actor rows
      // may instantiate them, at the server's current square.
      const scenery: typeof plan.structures = [];
      const sources = new Map<string, THREE.Group>();
      plan.structures.forEach((placement, index) => {
        if (placement.file.startsWith("resident_")) return;
        sources.set(`structures/${level}/${scenery.length}`, this.structures.get(`structures/${level}/${index}`)!);
        scenery.push(placement);
      });
      this.active = new SpaceScene({ name: level, space: { ...plan, structures: scenery }, textures: this.textures,
        windWeightTextures: this.wind, presets: ["day"], anisotropy: this.renderer.capabilities.getMaxAnisotropy(),
        camera: this.camera, caretakerCell: { i: at.x, j: at.y }, caretakerFacing: { i: 0, j: 1 }, figures: this.figures,
        caretakerFigure: this.packet.manifest.caretaker.figure, structures: sources });
      this.scene.add(this.active.group); this.scene.background = this.active.background; this.level = level;
    }
    const previous = this.snapshot?.envelope.frame.observation_center.position;
    const player = this.active!.caretaker;
    if (previous) {
      const dx = at.x - previous.x, dy = at.y - previous.y;
      if (dx || dy) player.setFacing({ i: Math.sign(dx), j: Math.sign(dy) });
    }
    if (previous && plan.weather) this.active!.focusLighting({ i: previous.x, j: previous.y }, { i: at.x, j: at.y });
    const sharing = frame.actors.some(actor => actor.actor_id !== frame.observer_actor_id &&
      (actor as typeof actor & { life_state: string }).life_state !== "dead" &&
      actor.position.position.x === at.x && actor.position.position.y === at.y);
    const displayed = { x: at.x - (sharing ? .23 : 0), y: at.y + (sharing ? .12 : 0) };
    if (previous && (at.x !== previous.x || at.y !== previous.y)) {
      const planned = this.walk?.kind === "committed" ? this.walk.route : null;
      const end = planned?.findIndex(cell => cell.i === at.x && cell.j === at.y) ?? -1;
      const route = end > 0 && planned?.[0]?.i === previous.x && planned[0].j === previous.y
        ? planned.slice(0,end+1).map(cell => ({ ...cell }))
        : [{ i:player.root.position.x,j:player.root.position.z },{ i:at.x,j:at.y }];
      route[route.length-1] = { i:displayed.x,j:displayed.y };
      this.motion = { route, started: performance.now() };
    } else if (!this.motion) player.place(displayed.x, displayed.y);
    focusFeelCamera(this.camera, focus);
    this.occupants.clear();
    const surface = createTerrainSurface(plan);
    this.residents.present(frame, level, plan, this.figures, this.structures,
      this.packet.manifest.caretaker.figure, studyReceipt.actor_figures);
    for (const row of [...frame.corpses, ...frame.ground_items, ...frame.gold_piles]) {
      const cell = row.location.position;
      const token = new THREE.Mesh(this.looseGeometry, this.looseMaterial);
      token.position.set(cell.x, surface.heightAt(cell.x, cell.y) + .14, cell.y); this.occupants.add(token);
    }
    this.snapshot = snapshot;
    this.canvas.dataset.studyLevel = level;
    this.canvas.dataset.studyActorCount = String(frame.actors.length);
  }

  pickActor(x: number, y: number): string | null {
    if (!this.snapshot || x < 0 || y < 0 || x >= this.width || y >= this.height) return null;
    const ray = new THREE.Raycaster();
    ray.setFromCamera(new THREE.Vector2(x / this.width * 2 - 1, 1 - y / this.height * 2), this.camera);
    const blocker = ray.intersectObject(this.active!.group, true).find(hit => {
      if (!(hit.object instanceof THREE.Mesh)) return false;
      const materials = Array.isArray(hit.object.material) ? hit.object.material : [hit.object.material];
      return materials.some(material => material.visible && material.opacity >= .9);
    });
    return this.residents.pick(ray, blocker?.distance);
  }

  presentWalk(walk: WalkPresentation): void {
    this.walk = walk;
    if (!this.level) { this.pathOverlay.clear(); return; }
    this.pathOverlay.present(walk, createTerrainSurface(this.packet.manifest.spaces[this.level]!).heightAt);
  }

  pointer(x: number, y: number): { coordinate: Coord } | null {
    if (!this.snapshot || x < 0 || y < 0 || x >= this.width || y >= this.height) return null;
    const ray = new THREE.Raycaster();
    ray.setFromCamera(new THREE.Vector2(x / this.width * 2 - 1, 1 - y / this.height * 2), this.camera);
    // Ground picking does not let foreground roofs or tree crowns move a target.
    const surface = createTerrainSurface(this.packet.manifest.spaces[this.level!]!);
    const point = new THREE.Vector3();
    if (!ray.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0,1,0),0),point)) return null;
    for (let index=0; index<4; index++) ray.ray.intersectPlane(new THREE.Plane(new THREE.Vector3(0,1,0),-surface.heightAt(point.x,point.z)),point);
    const coordinate = { x: Math.round(point.x), y: Math.round(point.z) };
    return this.snapshot.envelope.frame.tiles.some(tile => tile.position.x === coordinate.x && tile.position.y === coordinate.y)
      ? { coordinate } : null;
  }

  dispose(): void {
    cancelAnimationFrame(this.animation); this.clear();
    this.pathOverlay.dispose();
    this.looseGeometry.dispose(); this.looseMaterial.dispose();
    for (const texture of this.wind.values()) texture.dispose();
    for (const decoded of this.textures.values()) decoded.texture.dispose();
    disposeDecodedFigures(this.figures); disposeFigureSources([...new Set(this.structures.values())]);
    this.renderer.dispose();
  }
}
