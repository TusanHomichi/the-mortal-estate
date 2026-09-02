import { CAMERA_OFFSET } from "../camera";
import type { PropPlacement } from "../feelTypes";

export const WALL_CARD_OFFSET = 0.01;

export interface PropCardTransform {
  position: { x: number; y: number; z: number };
  rotationY: number;
  contactShadowRotation: {
    order: "YXZ";
    x: number;
    y: number;
    z: number;
  };
  scaleX: 1 | -1;
}

/**
 * Resolves packet-authored card facing without consulting the camera at render
 * time. Wall cards stay in their wall plane; mirror only flips the card's local
 * horizontal axis, so it cannot move the card off that plane.
 */
export function propCardTransform(placement: PropPlacement): PropCardTransform {
  const [i, j] = placement.cell_anchor;
  const base = {
    position: { x: i, y: placement.elevation + placement.nominal_height / 2, z: j },
    contactShadowRotation: { order: "YXZ" as const, x: -Math.PI / 2, y: 0, z: 0 },
    scaleX: placement.mirror ? -1 as const : 1 as const,
  };

  if (placement.facing === "+z") {
    return {
      ...base,
      position: { ...base.position, z: j - 0.5 + WALL_CARD_OFFSET },
      rotationY: 0,
    };
  }
  if (placement.facing === "+x") {
    return {
      ...base,
      position: { ...base.position, x: i - 0.5 + WALL_CARD_OFFSET },
      rotationY: Math.PI / 2,
      contactShadowRotation: { ...base.contactShadowRotation, y: Math.PI / 2 },
    };
  }
  return {
    ...base,
    rotationY: Math.atan2(CAMERA_OFFSET.x, CAMERA_OFFSET.z),
  };
}
