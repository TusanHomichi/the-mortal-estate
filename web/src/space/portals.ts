import type { FeelSpace, PortalTarget } from "../feelTypes";
import type { Cell } from "../walk/layoutPassability";

export function portalLandingFor(
  space: FeelSpace,
  route: readonly Cell[],
): PortalTarget | null {
  const landing = route.at(-1);
  if (landing === undefined) return null;
  return space.portals.find(
    (portal) => portal.cell[0] === landing.i && portal.cell[1] === landing.j,
  )?.to ?? null;
}
