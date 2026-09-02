import { OrthographicCamera, Vector3 } from "three";

export const CAMERA_VERTICAL_SIZE_1280X800 = 5.050762722761054;
export const CAMERA_TARGET_HEIGHT = 1.22;
export const CAMERA_OFFSET = new Vector3(8, 6.531973, 8);
const CAMERA_REFERENCE_VIEWPORT_HEIGHT = 800;

export interface FeelCameraFocus {
  i: number;
  j: number;
}

export function createFeelCamera(
  width: number,
  height: number,
  initialFocus: FeelCameraFocus,
): OrthographicCamera {
  const aspect = width / height;
  const halfHeight = CAMERA_VERTICAL_SIZE_1280X800 / 2;
  const camera = new OrthographicCamera(
    -halfHeight * aspect,
    halfHeight * aspect,
    halfHeight,
    -halfHeight,
    0.1,
    100,
  );
  camera.updateProjectionMatrix();
  focusFeelCamera(camera, initialFocus);
  return camera;
}

export function focusFeelCamera(camera: OrthographicCamera, cell: FeelCameraFocus): void {
  const target = new Vector3(cell.i, CAMERA_TARGET_HEIGHT, cell.j);
  camera.position.copy(target).add(CAMERA_OFFSET);
  camera.lookAt(target);
  camera.updateMatrixWorld(true);
}

export function resizeFeelCamera(camera: OrthographicCamera, width: number, height: number): void {
  const halfHeight = CAMERA_VERTICAL_SIZE_1280X800 / 2;
  const halfWidth = halfHeight * (width / height);
  camera.left = -halfWidth;
  camera.right = halfWidth;
  camera.top = halfHeight;
  camera.bottom = -halfHeight;
  camera.updateProjectionMatrix();
}

export function projectedCellDiamondWidth(camera: OrthographicCamera, viewportWidth: number): number {
  const corners = [
    new Vector3(-0.5, 0, -0.5),
    new Vector3(0.5, 0, -0.5),
    new Vector3(0.5, 0, 0.5),
    new Vector3(-0.5, 0, 0.5),
  ];
  const screenX = corners.map((corner) => (corner.project(camera).x + 1) * viewportWidth * 0.5);
  return Math.max(...screenX) - Math.min(...screenX);
}

/**
 * How many ground tiles the projected height of a world-up object covers in
 * the camera's away-from-camera direction. Both ground axes have the same
 * projection under the ruled 45-degree yaw; using the smaller measured depth
 * keeps the calculation correct if that symmetry changes by rounding.
 */
export function projectedHeightCoverTiles(height: number): number {
  const cameraDistance = CAMERA_OFFSET.length();
  const horizontalDistance = Math.hypot(CAMERA_OFFSET.x, CAMERA_OFFSET.z);
  const pixelsPerCameraUnit =
    CAMERA_REFERENCE_VIEWPORT_HEIGHT / CAMERA_VERTICAL_SIZE_1280X800;
  const heightPixelsPerWorldUnit =
    (horizontalDistance / cameraDistance) * pixelsPerCameraUnit;
  const depthPixelsAlongI =
    Math.abs(
      (CAMERA_OFFSET.x * CAMERA_OFFSET.y) /
        (cameraDistance * horizontalDistance),
    ) * pixelsPerCameraUnit;
  const depthPixelsAlongJ =
    Math.abs(
      (CAMERA_OFFSET.z * CAMERA_OFFSET.y) /
        (cameraDistance * horizontalDistance),
    ) * pixelsPerCameraUnit;
  return (
    height * heightPixelsPerWorldUnit /
    Math.min(depthPixelsAlongI, depthPixelsAlongJ)
  );
}
