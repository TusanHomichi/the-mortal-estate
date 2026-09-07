import { OrthographicCamera, Vector3 } from "three";

/** Nine ground-cell rows tall at the accepted 45-degree elevation. */
export const CAMERA_VERTICAL_SIZE_1280X800 = 9 * Math.sin(Math.PI / 4);
export const CAMERA_TARGET_HEIGHT = 1.22;
export const CAMERA_OFFSET = new Vector3(0, 8, 8);
const DIMETRIC_OFFSET = new Vector3(8, 6.531973, 8);
export type CameraView = "dimetric" | "front" | "front-high";
const cameraViews = new WeakMap<OrthographicCamera, CameraView>();
/** Owner comparison: same elevation and scale, upright world, zero yaw. */
const FRONT_OFFSET = new Vector3(0, DIMETRIC_OFFSET.y, Math.hypot(DIMETRIC_OFFSET.x, DIMETRIC_OFFSET.z));
export function cameraViewFromUrl(url: URL): CameraView {
  const value=url.searchParams.get("view") ?? "front-high";
  if(value!=="dimetric"&&value!=="front"&&value!=="front-high") throw new Error(`unknown camera comparison ${value}`);
  return value;
}

/**
 * A comparison zoom for the owner's viewport ruling: each step multiplies the
 * frame's world height by this ratio, negative steps outward. Step 0 is the
 * ruled frame; every other step is a candidate under review, not a default.
 */
export const CAMERA_ZOOM_STEP_RATIO = 1.25;
export const CAMERA_ZOOM_STEP_LIMIT = 3;

export function cameraVerticalSize(zoomStep: number): number {
  if (!Number.isInteger(zoomStep) || Math.abs(zoomStep) > CAMERA_ZOOM_STEP_LIMIT) {
    throw new Error(`zoom step ${zoomStep} is outside the comparison range`);
  }
  return CAMERA_VERTICAL_SIZE_1280X800 * CAMERA_ZOOM_STEP_RATIO ** -zoomStep;
}

export interface FeelCameraFocus {
  i: number;
  j: number;
}

/** The minimum a space must say for the camera to know whom it belongs to. */
export interface CameraSpace {
  grid_extents: { i: number; j: number };
  weather: boolean;
}

/** The centre of a space's grid, as a focus: cell (0,0) to (i-1, j-1). */
export function spaceCameraFocus(extents: CameraSpace["grid_extents"]): FeelCameraFocus {
  return { i: (extents.i - 1) / 2, j: (extents.j - 1) / 2 };
}

/**
 * Whom the camera belongs to (owner ruling, 2026-09-02): inside a building
 * the camera belongs to the space — centred on it, and it stops following
 * the character; outside it stays centred on the player and re-centres on
 * each landing. A dedicated interior is the space that carries no weather.
 */
export function cameraFollowsCaretaker(space: CameraSpace): boolean {
  return space.weather;
}

export function cameraFocusFor(space: CameraSpace, caretakerCell: FeelCameraFocus): FeelCameraFocus {
  return cameraFollowsCaretaker(space) ? caretakerCell : spaceCameraFocus(space.grid_extents);
}

export function createFeelCamera(
  width: number,
  height: number,
  initialFocus: FeelCameraFocus,
  zoomStep = 0,
  view: CameraView = "front-high",
): OrthographicCamera {
  const aspect = width / height;
  const halfHeight = cameraVerticalSize(zoomStep) / 2;
  const camera = new OrthographicCamera(
    -halfHeight * aspect,
    halfHeight * aspect,
    halfHeight,
    -halfHeight,
    0.1,
    100,
  );
  camera.updateProjectionMatrix();
  cameraViews.set(camera, view);
  focusFeelCamera(camera, initialFocus);
  return camera;
}

export function focusFeelCamera(camera: OrthographicCamera, cell: FeelCameraFocus): void {
  const target = new Vector3(cell.i, CAMERA_TARGET_HEIGHT, cell.j);
  camera.position.copy(target).add(cameraViews.get(camera) === "dimetric" ? DIMETRIC_OFFSET : cameraViews.get(camera) === "front" ? FRONT_OFFSET : CAMERA_OFFSET);
  camera.lookAt(target);
  camera.updateMatrixWorld(true);
}

export function resizeFeelCamera(
  camera: OrthographicCamera,
  width: number,
  height: number,
  zoomStep = 0,
): void {
  const halfHeight = cameraVerticalSize(zoomStep) / 2;
  const halfWidth = halfHeight * (width / height);
  camera.left = -halfWidth;
  camera.right = halfWidth;
  camera.top = halfHeight;
  camera.bottom = -halfHeight;
  camera.updateProjectionMatrix();
}

export function projectedCellWidth(camera: OrthographicCamera, viewportWidth: number): number {
  const corners = [
    new Vector3(-0.5, 0, -0.5),
    new Vector3(0.5, 0, -0.5),
    new Vector3(0.5, 0, 0.5),
    new Vector3(-0.5, 0, 0.5),
  ];
  const screenX = corners.map((corner) => (corner.project(camera).x + 1) * viewportWidth * 0.5);
  return Math.max(...screenX) - Math.min(...screenX);
}

/** Projected world-up height expressed in ground rows away from the camera. */
export function projectedHeightCoverTiles(height: number): number {
  return height * Math.hypot(CAMERA_OFFSET.x, CAMERA_OFFSET.z) / CAMERA_OFFSET.y;
}
