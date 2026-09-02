/**
 * Browser presentation for the feel scene's walk experiment.
 *
 * This is local and non-authoritative: packet-layout passability is a visual
 * guess and the beat is a stand-in for the server-owned pulse. The module is
 * intentionally the walk feature's only Three.js and DOM boundary.
 */
import {
  CanvasTexture,
  Color,
  LineBasicMaterial,
  LineLoop,
  Mesh,
  MeshBasicMaterial,
  PlaneGeometry,
  SRGBColorSpace,
  Scene,
  BufferGeometry,
  Float32BufferAttribute,
  type OrthographicCamera,
} from "three";
import type { FeelLayout } from "../feelTypes";
import { BeatClock } from "./beat";
import { footprintsFromPath } from "./footprints";
import { findPath } from "./pathfinding";
import { cellUnderPointer } from "./pointer";
import { passabilityFrom, type Cell } from "./layoutPassability";
import {
  advanceWalk,
  cancelWalk,
  createWalkIntent,
  doubleClick,
  planningOrigin,
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

interface DrawnFootprint {
  mesh: Mesh<PlaneGeometry, MeshBasicMaterial>;
  pathIndex: number;
  kind: "preview" | "committed";
}

const PREVIEW_COLOUR = new Color("#9eb5ca");
const COMMITTED_COLOUR = new Color("#c8c3b8");

function caretakerCell(layout: FeelLayout): Cell {
  const caretaker = layout.props.find((prop) => prop.kind === "caretaker");
  if (caretaker === undefined) throw new Error("the walk experiment needs one caretaker placement");
  return { i: Math.round(caretaker.cell_anchor[0]), j: Math.round(caretaker.cell_anchor[1]) };
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
  outline.name = "WalkReachableCell";
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
    path: readonly Cell[],
    kind: DrawnFootprint["kind"],
    colour: Color,
    opacity: number,
  ): void => {
    for (const pair of footprintsFromPath(path)) {
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
        const kindName = kind === "preview" ? "Preview" : "Committed";
        mesh.name = `Walk${kindName}Footprint_${pair.pathIndex}`;
        mesh.rotation.x = -Math.PI / 2;
        mesh.rotation.z = -pair.angle;
        mesh.position.set(point.x, kind === "preview" ? 0.007 : 0.006, point.z);
        mesh.renderOrder = kind === "preview" ? 4 : 3;
        scene.add(mesh);
        footprints.push({ mesh, pathIndex: pair.pathIndex, kind });
      }
    }
  };

  const syncFootprints = (): void => {
    const committedIdentity = state.committed?.path
      .map((cell) => `${cell.i},${cell.j}`)
      .join(";") ?? "";
    const previewIdentity =
      state.preview?.map((cell) => `${cell.i},${cell.j}`).join(";") ?? "";
    const identity = `${committedIdentity}|${previewIdentity}`;
    if (identity === footprintIdentity) return;
    footprintIdentity = identity;
    clearFootprints();
    if (state.committed !== null) {
      drawFootprints(state.committed.path, "committed", COMMITTED_COLOUR, 0.85);
    }
    if (state.preview !== null) {
      drawFootprints(state.preview, "preview", PREVIEW_COLOUR, 0.45);
    }
  };

  const updateHover = (): void => {
    if (hoverCell === null) {
      hoverOutline.visible = false;
      return;
    }
    const reachable = findPath(passability, planningOrigin(state), hoverCell) !== null;
    hoverOutline.visible = reachable;
    if (reachable) hoverOutline.position.set(hoverCell.i, 0, hoverCell.j);
  };

  const reflectState = (): void => {
    stage.dataset.walkState = walkIntentKind(state);
    stage.dataset.caretakerCell = `${state.caretakerCell.i},${state.caretakerCell.j}`;
    syncFootprints();
    updateHover();
  };

  const transition = (next: WalkIntentState): void => {
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
    transition(singleClick(state, passability, target, nowSeconds()));
  };
  const onDoubleClick = (event: MouseEvent): void => {
    if (event.button !== 0) return;
    event.preventDefault();
    transition(doubleClick(state, passability, nowSeconds()));
  };
  const onContextMenu = (event: MouseEvent): void => {
    event.preventDefault();
    transition(cancelWalk(state, passability, nowSeconds()));
  };
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") return;
    transition(cancelWalk(state, passability, nowSeconds()));
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
      const advanced = advanceWalk(state, passability, now);
      if (advanced !== state) transition(advanced);
      const position = presentedCaretakerPosition(state, now);
      caretaker.card.position.set(position.i, cardHeight, position.j);
      caretaker.contactShadow.position.set(position.i, shadowHeight, position.j);
      if (state.activeStep !== null) {
        const directionI = state.activeStep.to.i - state.activeStep.from.i;
        if (directionI < 0) caretaker.card.scale.x = -homeScaleX;
        if (directionI > 0) caretaker.card.scale.x = homeScaleX;
      }

      const beat = state.activeStep === null ? standInClock : new BeatClock(state.activeStep.startedAt);
      const phase = beat.phase(now);
      meterFill.style.transform = `scaleX(${phase})`;

      if (state.committed !== null) {
        for (const footprint of footprints) {
          if (footprint.kind !== "committed") continue;
          if (footprint.pathIndex < state.committed.stepIndex - 1) {
            footprint.mesh.material.opacity = 0;
          } else if (footprint.pathIndex === state.committed.stepIndex - 1) {
            footprint.mesh.material.opacity = 0.85 * (1 - phase);
          } else {
            footprint.mesh.material.opacity = 0.85;
          }
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
