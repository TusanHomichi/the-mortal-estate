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
