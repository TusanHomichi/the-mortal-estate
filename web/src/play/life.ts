import type { Frame } from "../authoritative/state";

/** Presentation uses the observed life state; only the server grants actions. */
export function observedLife(frame: Frame | undefined): string | undefined {
  return frame?.actors.find(actor => actor.actor_id === frame.observer_actor_id)?.life_state;
}

export function physicalControlsAvailable(frame: Frame | undefined): boolean {
  return !!frame?.can_act && observedLife(frame) === "alive";
}
