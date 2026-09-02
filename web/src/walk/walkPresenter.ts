/**
 * Browser presentation for the feel scene's walk experiment.
 *
 * This is local and non-authoritative: packet-layout passability is a visual
 * guess and the beat is a stand-in for the server-owned pulse. The module is
 * intentionally the walk feature's only Three.js and DOM boundary.
 */
import {
  BufferGeometry,
  CanvasTexture,
  Color,
  Float32BufferAttribute,
  LineBasicMaterial,
  LineLoop,
  Mesh,
  MeshBasicMaterial,
  PlaneGeometry,
  Scene,
  SRGBColorSpace,
  type OrthographicCamera,
} from "three";
import type { FeelLayout } from "../feelTypes";
import { BeatClock, WALK_STAND_IN_BEAT_SECONDS } from "./beat";
import { footprintsFromPath } from "./footprints";
import { passabilityFrom, type Cell } from "./layoutPassability";
import { cellUnderPointer } from "./pointer";
import { authorRoute } from "./route";
import {
  advanceWalk,
  cancelWalk,
  createWalkIntent,
  doubleClick,
  presentedCaretakerPosition,
  singleClick,
  walkIntentKind,
  type WalkIntentState,
} from "./walkIntent";

interface CaretakerObjects {
  card: Mesh;
  contactShadow: Mesh;
}

export interface WalkPresenterOptions {
  stage: HTMLElement;
  canvas: HTMLCanvasElement;
  scene: Scene;
  camera: OrthographicCamera;
  layout: FeelLayout;
  caretaker: CaretakerObjects;
  startedAt: number;
}

export interface WalkPresenter {
  update(now: number): void;
  stop(): void;
}

type FootprintKind = "draft" | "committed" | "landed";

interface DrawnFootprint {
  mesh: Mesh<PlaneGeometry, MeshBasicMaterial>;
  kind: FootprintKind;
}

interface LandedFootprints {
  route: Cell[];
  landedAt: number;
}

const DRAFT_COLOUR = new Color("#9eb5ca");
const COMMITTED_COLOUR = new Color("#c8c3b8");

function caretakerCell(layout: FeelLayout): Cell {
  const caretaker = layout.props.find((prop) => prop.kind === "caretaker");
  if (caretaker === undefined) throw new Error("the walk experiment needs one caretaker placement");
  return { i: Math.round(caretaker.cell_anchor[0]), j: Math.round(caretaker.cell_anchor[1]) };
}

function routeIdentity(route: readonly Cell[] | null): string {
  return route?.map((cell) => `${cell.i},${cell.j}`).join(";") ?? "";
}

function makeSoleTexture(): CanvasTexture {
  const drawing = document.createElement("canvas");
  drawing.width = 48;
  drawing.height = 80;
  const context = drawing.getContext("2d");
  if (context === null) throw new Error("the walk experiment could not draw its footprints");
  context.clearRect(0, 0, drawing.width, drawing.height);
  context.filter = "blur(2px)";
  context.fillStyle = "rgba(255, 255, 255, 0.92)";
  context.beginPath();
  context.ellipse(24, 24, 13, 18, 0, 0, Math.PI * 2);
  context.ellipse(24, 54, 9, 18, 0, 0, Math.PI * 2);
  context.fill();
  context.filter = "none";
  const texture = new CanvasTexture(drawing);
  texture.colorSpace = SRGBColorSpace;
  texture.needsUpdate = true;
  return texture;
}

function makeHoverOutline(): LineLoop<BufferGeometry, LineBasicMaterial> {
  const geometry = new BufferGeometry();
  geometry.setAttribute(
    "position",
    new Float32BufferAttribute(
      [
        -0.5, 0.008, -0.5,
        0.5, 0.008, -0.5,
        0.5, 0.008, 0.5,
        -0.5, 0.008, 0.5,
      ],
      3,
    ),
  );
  const outline = new LineLoop(
    geometry,
    new LineBasicMaterial({ color: 0xa6b8c9, transparent: true, opacity: 0.72 }),
  );
  outline.name = "WalkAuthorableCell";
  outline.visible = false;
  outline.renderOrder = 4;
  return outline;
}

export function createWalkPresenter(options: WalkPresenterOptions): WalkPresenter {
  const { stage, canvas, scene, camera, layout, caretaker } = options;
  const passability = passabilityFrom(layout);
  const standInClock = new BeatClock(options.startedAt);
  const homeScaleX = Math.abs(caretaker.card.scale.x || 1);
  const cardHeight = caretaker.card.position.y;
  const shadowHeight = caretaker.contactShadow.position.y;
  const soleTexture = makeSoleTexture();
  const hoverOutline = makeHoverOutline();
  scene.add(hoverOutline);

  const label = document.createElement("div");
  label.className = "walk-experiment-label";
  label.textContent = "WALK EXPERIMENT — LOCAL, NOT AUTHORITY";
  label.setAttribute("aria-hidden", "true");
  stage.append(label);

  const meter = document.createElement("div");
  meter.className = "walk-beat-meter";
  meter.setAttribute("aria-hidden", "true");
  const meterFill = document.createElement("div");
  meterFill.className = "walk-beat-meter__fill";
  meter.append(meterFill);
  stage.append(meter);

  let state = createWalkIntent(caretakerCell(layout));
  let footprints: DrawnFootprint[] = [];
  let footprintIdentity = "";
  let landedFootprints: LandedFootprints | null = null;
  let hoverCell: Cell | null = null;

  const clearFootprints = (): void => {
    for (const footprint of footprints) {
      scene.remove(footprint.mesh);
      footprint.mesh.geometry.dispose();
      footprint.mesh.material.dispose();
    }
    footprints = [];
  };

  const drawFootprints = (
    route: readonly Cell[],
    kind: FootprintKind,
    colour: Color,
    opacity: number,
  ): void => {
    for (const pair of footprintsFromPath(route)) {
      for (const point of [pair.left, pair.right]) {
        const material = new MeshBasicMaterial({
          map: soleTexture,
          color: colour.clone(),
          transparent: true,
          opacity,
          depthWrite: false,
          polygonOffset: true,
          polygonOffsetFactor: -2,
        });
        const mesh = new Mesh(new PlaneGeometry(0.12, 0.2), material);
        mesh.name = `Walk${kind[0]!.toUpperCase()}${kind.slice(1)}Footprint_${pair.pathIndex}`;
        mesh.rotation.x = -Math.PI / 2;
        mesh.rotation.z = -pair.angle;
        mesh.position.set(point.x, kind === "draft" ? 0.007 : 0.006, point.z);
        mesh.renderOrder = kind === "draft" ? 4 : 3;
        scene.add(mesh);
        footprints.push({ mesh, kind });
      }
    }
  };

  const syncFootprints = (): void => {
    const identity = [
      routeIdentity(landedFootprints?.route ?? null),
      routeIdentity(state.committed?.route ?? null),
      routeIdentity(state.draft),
    ].join("|");
    if (identity === footprintIdentity) return;
    footprintIdentity = identity;
    clearFootprints();
    if (landedFootprints !== null) {
      drawFootprints(landedFootprints.route, "landed", COMMITTED_COLOUR, 0.85);
    }
    if (state.committed !== null) {
      drawFootprints(state.committed.route, "committed", COMMITTED_COLOUR, 0.85);
    }
    if (state.draft !== null) {
      drawFootprints(state.draft, "draft", DRAFT_COLOUR, 0.45);
    }
  };

  const updateHover = (): void => {
    if (hoverCell === null) {
      hoverOutline.visible = false;
      return;
    }
    const authorable = authorRoute(passability, state.caretakerCell, hoverCell) !== null;
    hoverOutline.visible = authorable;
    if (authorable) hoverOutline.position.set(hoverCell.i, 0, hoverCell.j);
  };

  const applyFacing = (): void => {
    if (state.committed === null) return;
    const route = state.committed.route;
    const directionI = route[route.length - 1]!.i - route[0]!.i;
    if (directionI < 0) caretaker.card.scale.x = -homeScaleX;
    if (directionI > 0) caretaker.card.scale.x = homeScaleX;
  };

  const reflectState = (): void => {
    const position = presentedCaretakerPosition(state);
    caretaker.card.position.set(position.i, cardHeight, position.j);
    caretaker.contactShadow.position.set(position.i, shadowHeight, position.j);
    applyFacing();
    stage.dataset.walkState = walkIntentKind(state);
    stage.dataset.caretakerCell = `${state.caretakerCell.i},${state.caretakerCell.j}`;
    syncFootprints();
    updateHover();
  };

  const transition = (next: WalkIntentState, now: number): void => {
    if (
      state.committed !== null &&
      next.committed === null &&
      now >= state.committed.landsAt
    ) {
      landedFootprints = {
        route: state.committed.route.map((cell) => ({ ...cell })),
        landedAt: state.committed.landsAt,
      };
    }
    state = next;
    reflectState();
  };

  const nowSeconds = (): number => performance.now() / 1000;

  const pointerCell = (event: MouseEvent | PointerEvent): Cell | null =>
    cellUnderPointer(camera, canvas, event.clientX, event.clientY, layout.grid_extents);

  const onPointerMove = (event: PointerEvent): void => {
    hoverCell = pointerCell(event);
    updateHover();
  };
  const onPointerLeave = (): void => {
    hoverCell = null;
    updateHover();
  };
  const onClick = (event: MouseEvent): void => {
    if (event.button !== 0 || event.detail !== 1) return;
    const target = pointerCell(event);
    if (target === null) return;
    const now = nowSeconds();
    transition(singleClick(state, passability, target, standInClock, now), now);
  };
  const onDoubleClick = (event: MouseEvent): void => {
    if (event.button !== 0) return;
    event.preventDefault();
    const now = nowSeconds();
    transition(doubleClick(state, standInClock, now), now);
  };
  const onContextMenu = (event: MouseEvent): void => {
    event.preventDefault();
    const now = nowSeconds();
    transition(cancelWalk(state, now), now);
  };
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") return;
    const now = nowSeconds();
    transition(cancelWalk(state, now), now);
  };

  canvas.addEventListener("pointermove", onPointerMove);
  canvas.addEventListener("pointerleave", onPointerLeave);
  canvas.addEventListener("click", onClick);
  canvas.addEventListener("dblclick", onDoubleClick);
  canvas.addEventListener("contextmenu", onContextMenu);
  window.addEventListener("keydown", onKeyDown);
  reflectState();

  return {
    update: (now: number): void => {
      const advanced = advanceWalk(state, now);
      if (advanced !== state) transition(advanced, now);

      meterFill.style.transform = `scaleX(${standInClock.phase(now)})`;

      if (landedFootprints !== null) {
        const fade = Math.max(
          0,
          1 - (now - landedFootprints.landedAt) / WALK_STAND_IN_BEAT_SECONDS,
        );
        for (const footprint of footprints) {
          if (footprint.kind === "landed") footprint.mesh.material.opacity = 0.85 * fade;
        }
        if (fade === 0) {
          landedFootprints = null;
          syncFootprints();
        }
      }
    },
    stop: (): void => {
      canvas.removeEventListener("pointermove", onPointerMove);
      canvas.removeEventListener("pointerleave", onPointerLeave);
      canvas.removeEventListener("click", onClick);
      canvas.removeEventListener("dblclick", onDoubleClick);
      canvas.removeEventListener("contextmenu", onContextMenu);
      window.removeEventListener("keydown", onKeyDown);
      clearFootprints();
      scene.remove(hoverOutline);
      hoverOutline.geometry.dispose();
      hoverOutline.material.dispose();
      soleTexture.dispose();
      label.remove();
      meter.remove();
      delete stage.dataset.walkState;
      delete stage.dataset.caretakerCell;
    },
  };
}
