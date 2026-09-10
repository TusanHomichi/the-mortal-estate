import { expect, it } from "vitest";
import { actorActionGroups } from "../src/play/actorInteraction";
import { offeredIntent } from "../src/authoritative/gameplay";
import type { Snapshot } from "../src/authoritative/state";

function snapshot(): Snapshot {
  return { generation: 8, envelope: { frame: {
    actors: [{ actor_id: "seller", name: "Same name", position: { position: { x: 2, y: 1 } } }],
    services_here: [
      { service_id: "balms", actor_id: "seller", name: "Supplies", capabilities: [
        { kind: "service_transaction", transactions: [{ label: "Balm", actions: [
          { id: "purchase", label: "Buy balm", enabled: true, blocked_reason: null,
            intent: { kind: "commit_service_transaction", service_id: "balms", transaction_id: "purchase" } },
        ] }] },
      ] },
      { service_id: "unrelated", actor_id: null, name: "Same name", capabilities: [] },
    ], npcs_here: [], action_options: [], action_options_truncated: false,
  } } } as unknown as Snapshot;
}

it("opens only the observed provider's projected services without guessing from names", () => {
  const state = snapshot(), before = structuredClone(state);
  expect(actorActionGroups(state, "seller").map(group => group.key)).toEqual(["service:balms"]);
  expect(actorActionGroups(state, "unobserved")).toEqual([]);
  expect(state).toEqual(before);
  state.envelope.frame.services_here = [];
  expect(actorActionGroups(state, "seller")).toEqual([]); // Out of service range.
});

it("disabled purchases and stale service positions do not turn into transactions", () => {
  const state = snapshot(), frame = state.envelope.frame;
  const action = actorActionGroups(state, "seller")[0]!.actions[0]!;
  expect(offeredIntent(frame, "service:balms", action.id)).toBe(action.intent);
  action.enabled = false; action.blocked_reason = "insufficient_gold";
  expect(actorActionGroups(state, "seller")[0]!.actions[0]!.blocked_reason).toBe("insufficient_gold");
  expect(offeredIntent(frame, "service:balms", action.id)).toBeNull();
  frame.services_here = [];
  expect(offeredIntent(frame, "service:balms", action.id)).toBeNull();
});

it("groups physical and actor-targeted spell offers by exact target, preserving authority", () => {
  const state = snapshot(), frame = state.envelope.frame;
  const offer = (id: string, intent: Record<string, unknown>) => ({ id, label: "Same name",
    enabled: true, blocked_reason: null, intent });
  frame.action_options = [
    offer("kick", { kind: "physical_attack", target_actor_id: "seller", mode: "kick", authorization: "safe" }),
    offer("jumpkick", { kind: "physical_attack", target_actor_id: "someone_else", mode: "jumpkick" }),
    offer("spell", { kind: "cast_spell", spell_id: "cure", target: { kind: "actor", actor_id: "seller" }, authorization: "safe" }),
    offer("warmed", { kind: "cast_warmed_spell", target: { kind: "actor", actor_id: "seller" }, authorization: "safe" }),
    offer("area", { kind: "cast_spell", target: { kind: "coordinate", position: { x: 2, y: 1 } } }),
    offer("self", { kind: "cast_spell", target: { kind: "self_target" } }),
    offer("untargeted", { kind: "cast_warmed_spell", target: null }),
    offer("unrelated", { kind: "warm_spell", target_actor_id: "seller" }),
  ] as typeof frame.action_options;
  const before = structuredClone(state);
  const groups = actorActionGroups(state, "seller");
  expect(groups.map(group => group.key)).toEqual(["service:balms", "character"]);
  expect(groups[1]!.actions.map(action => action.id)).toEqual(["kick", "spell", "warmed"]);
  for (const action of groups[1]!.actions) expect(offeredIntent(frame, "character", action.id)).toBe(action.intent);
  expect(state).toEqual(before);
  frame.actors = [];
  expect(actorActionGroups(state, "seller")).toEqual([]);
});

it("keeps blocked combat offers visible and refuses withdrawn or ambiguous offers", () => {
  const state = snapshot(), frame = state.envelope.frame;
  frame.services_here = [];
  const attack = { id: "fight", label: "fight Same name", enabled: false, blocked_reason: "out_of_range",
    intent: { kind: "physical_attack", target_actor_id: "seller", mode: "fight", authorization: "safe" } };
  frame.action_options = [attack] as typeof frame.action_options;
  expect(actorActionGroups(state, "seller")[0]!.actions[0]).toBe(attack);
  expect(offeredIntent(frame, "character", "fight")).toBeNull();
  attack.enabled = true;
  frame.action_options.push({ ...attack });
  expect(offeredIntent(frame, "character", "fight")).toBeNull();
  frame.action_options = [];
  expect(actorActionGroups(state, "seller")).toEqual([]);
  expect(offeredIntent(frame, "character", "fight")).toBeNull();
});

it("offers enabled stairs only to the observed actor, using the supplied intent", () => {
  const state = snapshot(), frame = state.envelope.frame;
  frame.actors.push({ ...frame.actors[0]!, actor_id: "observer", name: "You" });
  frame.observer_actor_id = "observer";
  frame.action_options = [
    { id: "stairs_up", label: "traverse stairs up", enabled: true, blocked_reason: null,
      intent: { kind: "traverse", traversal: "stairs_up" } },
    { id: "climb_down", label: "traverse climb down", enabled: false, blocked_reason: "no_route",
      intent: { kind: "traverse", traversal: "climb_down" } },
  ] as typeof frame.action_options;
  const before = structuredClone(state);
  const groups = actorActionGroups(state, "observer");
  expect(groups.map(group => group.key)).toEqual(["character"]);
  expect(groups[0]!.actions.map(action => action.id)).toEqual(["stairs_up"]);
  const offered = groups[0]!.actions[0]!;
  expect(offered.intent).toBe(frame.action_options[0]!.intent);
  expect(offeredIntent(frame, "character", "stairs_up")).toBe(offered.intent);
  expect(offeredIntent(frame, "character", "climb_down")).toBeNull();
  // Another actor is not the observer, so traversal is never offered for them.
  expect(actorActionGroups(state, "seller").map(group => group.key)).toEqual(["service:balms"]);
  expect(actorActionGroups(state, "seller").flatMap(group => group.actions)
    .some(action => action.intent?.kind === "traverse")).toBe(false);
  expect(state).toEqual(before);
});

it("refuses withdrawn, ambiguous, or disabled traversal without inventing an intent", () => {
  const state = snapshot(), frame = state.envelope.frame;
  frame.actors.push({ ...frame.actors[0]!, actor_id: "observer", name: "You" });
  frame.observer_actor_id = "observer";
  frame.services_here = [];
  const stairs = { id: "stairs_up", label: "traverse stairs up", enabled: true, blocked_reason: null,
    intent: { kind: "traverse", traversal: "stairs_up" } };
  frame.action_options = [stairs] as typeof frame.action_options;
  expect(offeredIntent(frame, "character", "stairs_up")).toBe(stairs.intent);
  frame.action_options = [];
  expect(actorActionGroups(state, "observer")).toEqual([]);
  expect(offeredIntent(frame, "character", "stairs_up")).toBeNull();
  frame.action_options = [stairs, { ...stairs }] as typeof frame.action_options;
  const before = structuredClone(state);
  expect(actorActionGroups(state, "observer")[0]!.actions).toHaveLength(2);
  expect(offeredIntent(frame, "character", "stairs_up")).toBeNull();
  expect(state).toEqual(before);
  frame.action_options = [{ ...stairs, enabled: false, blocked_reason: "no_stairs" }] as typeof frame.action_options;
  expect(actorActionGroups(state, "observer")).toEqual([]);
  expect(offeredIntent(frame, "character", "stairs_up")).toBeNull();
  expect(frame.action_options[0]!.intent).toEqual({ kind: "traverse", traversal: "stairs_up" });
});

it("does not offer traversal when a minimal frame has no observer identity", () => {
  const state = snapshot(), frame = state.envelope.frame;
  frame.action_options = [{ id: "stairs_up", label: "traverse stairs up", enabled: true, blocked_reason: null,
    intent: { kind: "traverse", traversal: "stairs_up" } }] as typeof frame.action_options;
  expect(actorActionGroups(state, "seller").flatMap(group => group.actions)
    .some(action => action.intent?.kind === "traverse")).toBe(false);
});
