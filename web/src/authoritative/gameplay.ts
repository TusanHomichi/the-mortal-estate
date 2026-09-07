// Presentation fields from the Rust-validated observer frame. This module does
// not decode wire data, assess actions, or calculate any gameplay quantity.
export interface OfferedAction {
  id: string; label: string; enabled: boolean; blocked_reason: string | null;
  intent: Readonly<{ kind: string; [field: string]: unknown }> | null;
}
interface Item { item_instance_id: string; name: string; quantity: number }
interface Actions { actions: OfferedAction[] }
export type ServiceCapability =
  | ({ kind: "skill_training"; selected_track_id: string | null } & Actions)
  | ({ kind: "skill_critique" | "spell_teaching" | "class_promotion" } & Actions)
  | { kind: "service_transaction"; transactions: (Actions & { label: string })[] }
  | { kind: "merchant"; listings: { item: Item; price_gold: string; purchase: OfferedAction }[];
      buy_all: OfferedAction; sales: OfferedAction[] }
  | { kind: "item_service" | "restoration"; operations: Actions[] }
  | { kind: "bank"; balance_gold: string; deposit_actions: OfferedAction[]; withdrawal_actions: OfferedAction[] }
  | { kind: "locker"; capacity: number; item_count: number; items: Item[];
      deposit_actions: OfferedAction[]; withdrawal_actions: OfferedAction[] };
export interface GameplayFields {
  character: {
    identity: { display_class: string };
    resources: { hp: number; max_hp: number; stamina: number; max_stamina: number; mp: number; max_mp: number };
    progression: { level: number; experience: string };
    skill_ledger: { track_id: string; track_display: string | null; level: number; critique_rank: number; level_title: string | null }[];
  };
  carried: { gold: { left_hand: string; right_hand: string; sack: string };
    items: { position: string; item: Item }[] };
  services_here: { service_id: string; actor_id: string | null; name: string; capabilities: ServiceCapability[] }[];
  npcs_here: { actor_id: string; name: string; interactions: Actions[] }[];
  action_options: OfferedAction[];
  action_options_truncated: boolean;
}
export interface ActionGroup { key: string; title: string; facts: string[]; actions: OfferedAction[] }

function capabilityActions(capability: ServiceCapability): OfferedAction[] {
  switch (capability.kind) {
    case "skill_training": case "skill_critique": case "spell_teaching": case "class_promotion": return capability.actions;
    case "service_transaction": return capability.transactions.flatMap(row => row.actions);
    case "merchant": return [...capability.listings.map(row => row.purchase), capability.buy_all, ...capability.sales];
    case "item_service": case "restoration": return capability.operations.flatMap(row => row.actions);
    case "bank": case "locker": return [...capability.deposit_actions, ...capability.withdrawal_actions];
  }
}
function capabilityFacts(capability: ServiceCapability): string[] {
  switch (capability.kind) {
    case "bank": return [`Bank balance: ${capability.balance_gold} gold`];
    case "locker": return [`Locker: ${capability.item_count} / ${capability.capacity} items`,
      ...capability.items.map(item => `${item.name} × ${item.quantity}`)];
    case "merchant": return capability.listings.map(row => `${row.item.name} × ${row.item.quantity}: ${row.price_gold} gold`);
    case "skill_training": return [`Training focus: ${capability.selected_track_id?.replaceAll("_", " ") ?? "no offered focus selected"}`];
    default: return [];
  }
}

/** These are precisely the assessed options supplied by this frame. Grouping
 * changes presentation only; callers must resolve the option again at dispatch. */
export function actionGroups(frame: GameplayFields): ActionGroup[] {
  const local: ActionGroup[] = [
    ...frame.services_here.map(service => ({ key: `service:${service.service_id}`, title: service.name,
      facts: service.capabilities.flatMap(capabilityFacts), actions: service.capabilities.flatMap(capabilityActions) })),
    ...frame.npcs_here.map(npc => ({ key: `npc:${npc.actor_id}`, title: npc.name,
      facts: [], actions: npc.interactions.flatMap(row => row.actions) })),
  ];
  const localActions = new Set(local.flatMap(group => group.actions.map(action => JSON.stringify([action.id, action.intent]))));
  return [...local, { key: "character", title: "Equipment and nearby actions", facts: frame.action_options_truncated
    ? ["The server returned a limited action list."] : [],
    actions: frame.action_options.filter(action => !localActions.has(JSON.stringify([action.id, action.intent]))) }];
}

export function offeredIntent(frame: GameplayFields, groupKey: string, actionId: string): OfferedAction["intent"] {
  const group = actionGroups(frame).find(row => row.key === groupKey);
  const matches = group?.actions.filter(row => row.id === actionId) ?? [];
  // Ambiguous identities cannot turn a displayed choice into a different action.
  return matches.length === 1 && matches[0]!.enabled ? matches[0]!.intent : null;
}

export function goldAmountField(intent: OfferedAction["intent"]): "offered_gold" | "amount" | "quantity" | null {
  switch (intent?.kind) {
    case "train": return "offered_gold";
    case "withdraw_bank_gold": return "amount";
    case "move_gold": return "quantity";
    default: return null;
  }
}

/** Only the user-entered quantity changes. The offered target, source and
 * service identities stay intact. Rust still validates bounds and legality. */
export function withGoldAmount(intent: NonNullable<OfferedAction["intent"]>, amount?: string): OfferedAction["intent"] {
  if (amount === undefined) return intent;
  const field = goldAmountField(intent);
  if (!field || !/^[1-9][0-9]{0,18}$/.test(amount) || BigInt(amount) > 9223372036854775807n) return null;
  return { ...intent, [field]: field === "quantity" ? { kind: "exact", amount } : amount };
}
