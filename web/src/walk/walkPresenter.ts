/**
 * Browser presentation for the feel scene's walk experiment.
 *
 * This is local and non-authoritative: packet-layout passability is a visual
 * guess and the beat is a stand-in for the server-owned pulse. The module is
 * intentionally the walk feature's only Three.js and DOM boundary.
 */
import {
  AdditiveBlending,
  BufferGeometry,
  CanvasTexture,
  Float32BufferAttribute,
  LineBasicMaterial,
  LineLoop,
  Mesh,
  MeshBasicMaterial,
  PlaneGeometry,
  Scene,
  SRGBColorSpace,
  Vector3,
  type DirectionalLight,
  type Object3D,
  type OrthographicCamera,
} from "three";
import { CAMERA_TARGET_HEIGHT, focusFeelCamera } from "../camera";
import type { FeelLayout } from "../feelTypes";
import { BeatClock, WALK_STAND_IN_BEAT_SECONDS } from "./beat";
import {
  WALK_CURSOR_HOTSPOT,
  walkCursorDataUris,
  type WalkCursorKind,
} from "./cursors";
import { footprintsFromPath } from "./footprints";
import { passabilityFrom, sameCell, type Cell } from "./layoutPassability";
import { cellUnderPointer } from "./pointer";
import { authorRoute } from "./route";
import {
  advanceWalk,
  cancelWalk,
  createWalkIntent,
  doubleClick,
  presentedCaretakerPosition,
  singleClick,
  walkPace,
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
  initialCell: Cell;
  keyLight: DirectionalLight;
  keyTarget: Object3D;
  updateWallFade: (playerCell: Cell, now: number) => number;
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

const DRAFT_FOOTPRINT_OPACITY = 0.7;
const COMMITTED_FOOTPRINT_OPACITY = 1;

function routeIdentity(route: readonly Cell[] | null): string {
  return route?.map((cell) => `${cell.i},${cell.j}`).join(";") ?? "";
}

function traceSole(context: CanvasRenderingContext2D): void {
  context.beginPath();
  context.moveTo(48, 12);
  context.bezierCurveTo(65, 12, 76, 24, 74, 42);
  context.bezierCurveTo(73, 54, 66, 60, 59, 66);
  context.bezierCurveTo(54, 72, 55, 82, 61, 94);
  context.bezierCurveTo(67, 108, 63, 127, 51, 132);
  context.bezierCurveTo(38, 137, 27, 127, 28, 113);
  context.bezierCurveTo(29, 101, 36, 92, 36, 81);
  context.bezierCurveTo(36, 72, 29, 66, 24, 57);
  context.bezierCurveTo(14, 40, 21, 20, 38, 14);
  context.bezierCurveTo(41, 13, 45, 12, 48, 12);
  context.closePath();
}

function paintSoleLayer(
  context: CanvasRenderingContext2D,
  colour: string,
  opacity: number,
  blur: number,
): void {
  context.save();
  // Canvas bottom becomes the print's toe after the ground-plane rotation.
  context.translate(16, 168);
  context.scale(1, -1);
  context.filter = `blur(${blur}px)`;
  context.globalAlpha = opacity;
  context.fillStyle = colour;
  traceSole(context);
  context.fill();
  context.restore();
}

function makeSoleTexture(kind: "draft" | "committed"): CanvasTexture {
  const drawing = document.createElement("canvas");
  drawing.width = 128;
  drawing.height = 192;
  const context = drawing.getContext("2d");
  if (context === null) throw new Error("the walk experiment could not draw its footprints");
  context.clearRect(0, 0, drawing.width, drawing.height);
  paintSoleLayer(context, "#8fb4ff", kind === "draft" ? 0.48 : 0.68, kind === "draft" ? 8 : 11);
  paintSoleLayer(context, "#dfeaff", 0.9, 2.4);
  paintSoleLayer(context, "#dfeaff", 0.55, 0.8);
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
  const {
    stage,
    canvas,
    scene,
    camera,
    layout,
    caretaker,
    keyLight,
    keyTarget,
    updateWallFade,
  } = options;
  const passability = passabilityFrom(layout);
  const standInClock = new BeatClock(options.startedAt);
  const homeScaleX = Math.abs(caretaker.card.scale.x || 1);
  const cardHeight = caretaker.card.position.y;
  const shadowHeight = caretaker.contactShadow.position.y;
  const soleTextures = {
    draft: makeSoleTexture("draft"),
    committed: makeSoleTexture("committed"),
  };
  const hoverOutline = makeHoverOutline();
  const cursorDataUris = walkCursorDataUris();
  scene.add(hoverOutline);

  const label = document.createElement("div");
  label.className = "walk-experiment-label";
  label.textContent = "WALK EXPERIMENT — LOCAL, NOT AUTHORITY";
  label.setAttribute("aria-hidden", "true");
  stage.append(label);

  let state = createWalkIntent(options.initialCell);
  let footprints: DrawnFootprint[] = [];
  let footprintIdentity = "";
  let landedFootprints: LandedFootprints | null = null;
  let hoverCell: Cell | null = null;
  stage.dataset.walkClockStartedAt = String(options.startedAt);
  stage.dataset.walkFadedRuns = String(
    updateWallFade(state.caretakerCell, options.startedAt),
  );

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
    opacity: number,
  ): void => {
    const texture = kind === "draft" ? soleTextures.draft : soleTextures.committed;
    for (const print of footprintsFromPath(route)) {
      const material = new MeshBasicMaterial({
        map: texture,
        transparent: true,
        opacity,
        blending: AdditiveBlending,
        depthWrite: false,
        polygonOffset: true,
        polygonOffsetFactor: -2,
        toneMapped: false,
      });
      const mesh = new Mesh(new PlaneGeometry(0.18, 0.27), material);
      mesh.name = `Walk${kind[0]!.toUpperCase()}${kind.slice(1)}Footprint_${print.printIndex}_${print.foot}`;
      mesh.rotation.x = -Math.PI / 2;
      mesh.rotation.z = print.angle;
      mesh.scale.x = print.foot === "left" ? -1 : 1;
      mesh.position.set(
        print.position.x,
        kind === "draft" ? 0.007 : 0.006,
        print.position.z,
      );
      mesh.renderOrder = kind === "draft" ? 4 : 3;
      scene.add(mesh);
      footprints.push({ mesh, kind });
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
      drawFootprints(landedFootprints.route, "landed", COMMITTED_FOOTPRINT_OPACITY);
    }
    if (state.committed !== null) {
      drawFootprints(state.committed.route, "committed", COMMITTED_FOOTPRINT_OPACITY);
    }
    if (state.draft !== null) {
      drawFootprints(state.draft, "draft", DRAFT_FOOTPRINT_OPACITY);
    }
  };

  const setCursor = (kind: WalkCursorKind | "default"): void => {
    stage.dataset.walkCursor = kind;
    if (kind === "default") {
      canvas.style.cursor = "default";
      return;
    }
    const fallback = kind === "waiting" ? "wait" : kind === "refused" ? "not-allowed" : "default";
    canvas.style.cursor = `url(${cursorDataUris[kind]}) ${WALK_CURSOR_HOTSPOT.x} ${WALK_CURSOR_HOTSPOT.y}, ${fallback}`;
  };

  const updateHover = (): void => {
    if (hoverCell === null) {
      hoverOutline.visible = false;
      stage.dataset.walkOutline = "hidden";
      setCursor("default");
      return;
    }
    const authorable = authorRoute(passability, state.caretakerCell, hoverCell) !== null;
    hoverOutline.visible = authorable;
    stage.dataset.walkOutline = authorable ? "visible" : "hidden";
    if (authorable) hoverOutline.position.set(hoverCell.i, 0, hoverCell.j);
    if (state.committed !== null) setCursor("waiting");
    else if (authorable || sameCell(state.caretakerCell, hoverCell)) setCursor("ready");
    else setCursor("refused");
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
    const pace = walkPace(state);
    label.textContent = `WALK EXPERIMENT — LOCAL, NOT AUTHORITY${pace === null ? "" : ` · ${pace.toUpperCase()}`}`;
    if (pace === null) delete stage.dataset.walkPace;
    else stage.dataset.walkPace = pace;
    const projection = new Vector3(
      state.caretakerCell.i,
      CAMERA_TARGET_HEIGHT,
      state.caretakerCell.j,
    ).project(camera);
    const bounds = canvas.getBoundingClientRect();
    stage.dataset.caretakerProjection = `${(projection.x + 1) * bounds.width * 0.5},${(1 - projection.y) * bounds.height * 0.5}`;
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
    if (!sameCell(state.caretakerCell, next.caretakerCell)) {
      const deltaI = next.caretakerCell.i - state.caretakerCell.i;
      const deltaJ = next.caretakerCell.j - state.caretakerCell.j;
      focusFeelCamera(camera, next.caretakerCell);
      keyLight.position.x += deltaI;
      keyLight.position.z += deltaJ;
      keyTarget.position.x += deltaI;
      keyTarget.position.z += deltaJ;
      keyLight.updateMatrixWorld(true);
      keyTarget.updateMatrixWorld(true);
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
      stage.dataset.walkFadedRuns = String(updateWallFade(state.caretakerCell, now));

      if (landedFootprints !== null) {
        const fade = Math.max(
          0,
          1 - (now - landedFootprints.landedAt) / WALK_STAND_IN_BEAT_SECONDS,
        );
        for (const footprint of footprints) {
          if (footprint.kind === "landed") {
            footprint.mesh.material.opacity = COMMITTED_FOOTPRINT_OPACITY * fade;
          }
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
      soleTextures.draft.dispose();
      soleTextures.committed.dispose();
      label.remove();
      canvas.style.cursor = "";
      delete stage.dataset.walkState;
      delete stage.dataset.walkPace;
      delete stage.dataset.walkCursor;
      delete stage.dataset.walkOutline;
      delete stage.dataset.walkClockStartedAt;
      delete stage.dataset.walkFadedRuns;
      delete stage.dataset.caretakerCell;
      delete stage.dataset.caretakerProjection;
    },
  };
}
