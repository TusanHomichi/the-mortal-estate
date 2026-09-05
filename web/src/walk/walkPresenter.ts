/**
 * Browser presentation for the feel scene's walk experiment.
 *
 * This is local and non-authoritative: packet-layout passability is a visual
 * guess and the cooldown is a stand-in for server-owned readiness. This module
 * owns Three.js presentation and cursor feedback.
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
  type OrthographicCamera,
} from "three";
import { CAMERA_TARGET_HEIGHT, focusFeelCamera } from "../camera";
import type { FeelSpace, PortalTarget } from "../feelTypes";
import { portalLandingFor } from "../space/portals";
import { WALK_MOVE_SECONDS } from "./movement";
import { facingBetween, type FigureFacing } from "./facing";
import type { FigureInstance } from "../space/figureRig";
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
  presentedWalkPosition,
  type PresentedWalk,
  singleClick,
  walkPace,
  walkIntentKind,
  type WalkIntentState,
} from "./walkIntent";


export interface WalkPresenterOptions {
  stage: HTMLElement;
  canvas: HTMLCanvasElement;
  scene: Scene;
  camera: OrthographicCamera;
  spaceName: string;
  space: FeelSpace;
  caretaker: FigureInstance;
  initialCell: Cell;
  updateWallFade: (playerCell: Cell, now: number) => number;
  onCellChanged: (previous: Cell, next: Cell) => void;
  /** Whether the camera re-centres on the caretaker at each landing (false inside a building). */
  cameraFollowsCaretaker: boolean;
  onPortalLanding: (target: PortalTarget, facing: FigureFacing) => void;
}

export interface WalkPresenter {
  update(now: number): void;
  stop(): void;
}

const PRESENTED_TRACE_FRAMES = 48;
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
    space,
    caretaker,
    updateWallFade,
  } = options;
  const passability = passabilityFrom(space);
  const soleTextures = {
    draft: makeSoleTexture("draft"),
    committed: makeSoleTexture("committed"),
  };
  const hoverOutline = makeHoverOutline();
  const cursorDataUris = walkCursorDataUris();
  scene.add(hoverOutline);

  const announcement = document.createElement("span");
  announcement.className = "movement-announcement";
  announcement.setAttribute("role", "status");
  announcement.setAttribute("aria-live", "polite");
  stage.append(announcement);

  let state = createWalkIntent(options.initialCell);
  let footprints: DrawnFootprint[] = [];
  let footprintIdentity = "";
  let landedFootprints: LandedFootprints | null = null;
  let hoverCell: Cell | null = null;
  stage.dataset.walkSpace = options.spaceName;
  stage.dataset.walkFadedRuns = String(
    updateWallFade(state.caretakerCell, performance.now() / 1000),
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
    if (stage.dataset.walkCursor !== kind) {
      announcement.textContent = kind === "waiting" ? "Movement cooling down."
        : kind === "refused" ? "That tile cannot be reached in this move."
        : "Ready to move.";
    }
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
      setCursor(state.committed !== null ? "waiting" : "default");
      return;
    }
    const authorable = state.committed === null && authorRoute(passability, state.caretakerCell, hoverCell) !== null;
    const refused = state.committed === null && !authorable && !sameCell(state.caretakerCell, hoverCell);
    hoverOutline.visible = authorable || refused;
    hoverOutline.material.color.set(refused ? 0xf18d85 : 0xa6b8c9);
    stage.dataset.walkOutline = hoverOutline.visible ? "visible" : "hidden";
    if (hoverOutline.visible) hoverOutline.position.set(hoverCell.i, 0, hoverCell.j);
    if (state.committed !== null) setCursor("waiting");
    else if (authorable || sameCell(state.caretakerCell, hoverCell)) setCursor("ready");
    else setCursor("refused");
  };

  // Record presented motion inside the page so the proof can inspect it even
  // when an external screenshot call spans the whole movement interval.
  const presentedTrace: string[] = [];
  const presentWalk = (now: number): PresentedWalk => {
    const presented = presentedWalkPosition(state, now);
    caretaker.place(presented.i, presented.j);
    if (presented.facing !== null) caretaker.setFacing(presented.facing);
    caretaker.setGait(presented.gait);
    stage.dataset.caretakerFacing = `${caretaker.facing.i},${caretaker.facing.j}`;
    stage.dataset.caretakerYaw = String(caretaker.root.rotation.y);
    stage.dataset.caretakerPresented = `${presented.i.toFixed(3)},${presented.j.toFixed(3)}`;
    stage.dataset.caretakerGait = presented.gait;
    stage.dataset.caretakerClip = caretaker.clip;
    presentedTrace.push(`${walkIntentKind(state)}/${presented.gait}@${presented.i.toFixed(3)},${presented.j.toFixed(3)}`);
    if (presentedTrace.length > PRESENTED_TRACE_FRAMES) presentedTrace.shift();
    stage.dataset.caretakerTrace = presentedTrace.join(" ");
    return presented;
  };

  const reflectState = (): void => {
    presentWalk(performance.now() / 1000);
    stage.dataset.walkState = walkIntentKind(state);
    if (state.committed !== null) {
      stage.dataset.walkCommittedAt = String(state.committed.committedAt);
      stage.dataset.walkLandsAt = String(state.committed.landsAt);
    } else {
      delete stage.dataset.walkCommittedAt;
      delete stage.dataset.walkLandsAt;
    }
    stage.dataset.caretakerCell = `${state.caretakerCell.i},${state.caretakerCell.j}`;
    const pace = walkPace(state);
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
      if (options.cameraFollowsCaretaker) {
        focusFeelCamera(camera, next.caretakerCell);
        stage.dataset.cameraFocus = `${next.caretakerCell.i},${next.caretakerCell.j}`;
      }
      options.onCellChanged(state.caretakerCell, next.caretakerCell);
    }
    state = next;
    reflectState();
  };

  const nowSeconds = (): number => performance.now() / 1000;

  const pointerCell = (event: MouseEvent | PointerEvent): Cell | null =>
    cellUnderPointer(camera, canvas, event.clientX, event.clientY, space.grid_extents);

  const onPointerMove = (event: PointerEvent): void => {
    hoverCell = pointerCell(event);
    if (state.committed === null && hoverCell !== null) {
      const facing = facingBetween(state.caretakerCell, hoverCell);
      if (facing !== null) caretaker.setFacing(facing);
    }
    updateHover();
  };
  const onPointerLeave = (): void => {
    hoverCell = null;
    updateHover();
  };
  // The frame update owns completion and portal crossings. Input must not
  // consume a landing first, even if a slow frame has passed the deadline.
  const onClick = (event: MouseEvent): void => {
    if (state.committed !== null) return;
    if (event.button !== 0 || event.detail !== 1) return;
    const target = pointerCell(event);
    if (target === null) return;
    const now = nowSeconds();
    transition(singleClick(state, passability, target, now), now);
  };
  const onDoubleClick = (event: MouseEvent): void => {
    if (event.button !== 0) return;
    event.preventDefault();
    const now = nowSeconds();
    if (state.committed === null) transition(doubleClick(state, now), now);
  };
  const onContextMenu = (event: MouseEvent): void => {
    event.preventDefault();
    const now = nowSeconds();
    if (state.committed === null) transition(cancelWalk(state, now), now);
  };
  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== "Escape") return;
    const now = nowSeconds();
    if (state.committed === null) transition(cancelWalk(state, now), now);
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
      if (advanced !== state) {
        // Preserve the terminal heading without emitting a committed/idle
        // presentation frame before the state transition has landed.
        const terminalFacing = presentedWalkPosition(state, now).facing;
        if (terminalFacing !== null) caretaker.setFacing(terminalFacing);
        const landing = state.committed === null
          ? null
          : portalLandingFor(space, state.committed.route);
        if (landing !== null) {
          options.onPortalLanding(landing, caretaker.facing);
          return;
        }
        transition(advanced, now);
      }
      // The wall fade follows the figure as presented, not the square it is
      // still logically on: walking into a wall's cover during movement must
      // fade that wall as soon as the figure reaches it.
      const presented = presentWalk(now);
      stage.dataset.walkFadedRuns = String(
        updateWallFade({ i: Math.round(presented.i), j: Math.round(presented.j) }, now),
      );

      if (landedFootprints !== null) {
        const fade = Math.max(
          0,
          1 - (now - landedFootprints.landedAt) / WALK_MOVE_SECONDS,
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
      announcement.remove();
      canvas.style.cursor = "";
      delete stage.dataset.walkState;
      delete stage.dataset.walkPace;
      delete stage.dataset.walkCursor;
      delete stage.dataset.walkOutline;
      delete stage.dataset.walkCommittedAt;
      delete stage.dataset.walkLandsAt;
      delete stage.dataset.walkSpace;
      delete stage.dataset.walkFadedRuns;
      delete stage.dataset.caretakerCell;
      delete stage.dataset.caretakerProjection;
      delete stage.dataset.caretakerPresented;
      delete stage.dataset.caretakerGait;
      delete stage.dataset.caretakerTrace;
      delete stage.dataset.caretakerFacing;
      delete stage.dataset.caretakerYaw;
    },
  };
}
