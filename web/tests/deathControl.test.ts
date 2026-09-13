import { afterEach, expect, it, vi } from "vitest";
import { DeathControl } from "../src/play/deathControl";
import type { ControlView } from "../src/play/control";

class Element {
  hidden = false; disabled = false; isConnected = true; textContent = "";
  children: Element[] = []; onclick: (() => void) | null = null;
  setAttribute() {} append(...children: Element[]) { this.children.push(...children); }
}
afterEach(() => vi.unstubAllGlobals());
const view = (): ControlView => ({ phase: "playing", busy: false, pending: false,
  snapshot: { generation: 1, envelope: { frame: { observer_actor_id: "player", can_act: false,
    actors: [{ actor_id: "player", life_state: "ghost" }], action_options: [
      { id: "return", label: "Request resurrection", enabled: false, blocked_reason: "not_ready", intent: { kind: "request_resurrection" } },
    ] } } } }) as unknown as ControlView;

it("requires the current eligible offer and revokes it on return or authority loss", () => {
  vi.stubGlobal("document", { createElement: () => new Element() });
  const root = new Element(), dispatch = vi.fn(() => true);
  const control = new DeathControl(root as unknown as HTMLElement, dispatch);
  const state = view(), frame = state.snapshot!.envelope.frame;
  control.present(state);
  const button = root.children[1]!;
  button.onclick!(); expect(dispatch).not.toHaveBeenCalled(); expect(root.hidden).toBe(false);
  frame.can_act = true;
  control.present(state); button.onclick!(); expect(dispatch).not.toHaveBeenCalled();
  frame.action_options[0]!.enabled = true; state.snapshot = { ...state.snapshot!, generation: 2 };
  control.present(state); button.onclick!(); expect(dispatch).toHaveBeenLastCalledWith(2, "character", "return");
  dispatch.mockClear(); state.pending = true;
  control.present(state); button.onclick!(); expect(dispatch).not.toHaveBeenCalled();
  state.pending = false; frame.actors[0]!.life_state = "alive";
  control.present(state); button.onclick!(); expect(dispatch).not.toHaveBeenCalled(); expect(root.hidden).toBe(true);
  frame.actors[0]!.life_state = "ghost"; state.phase = "disconnected";
  control.present(state); button.onclick!(); expect(dispatch).not.toHaveBeenCalled(); expect(root.hidden).toBe(true);
});

it("retires a removed offer and refuses a detached control", () => {
  vi.stubGlobal("document", { createElement: () => new Element() });
  const root = new Element(), dispatch = vi.fn(() => true);
  const control = new DeathControl(root as unknown as HTMLElement, dispatch), state = view();
  state.snapshot!.envelope.frame.can_act = true; state.snapshot!.envelope.frame.action_options[0]!.enabled = true;
  control.present(state); const button = root.children[1]!;
  button.isConnected = false; button.onclick!(); expect(dispatch).not.toHaveBeenCalled();
  button.isConnected = true; state.snapshot!.envelope.frame.action_options = [];
  control.present(state); button.onclick!(); expect(dispatch).not.toHaveBeenCalled(); expect(button.hidden).toBe(true);
});
