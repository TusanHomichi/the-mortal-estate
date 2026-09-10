import { actionGroups, type ActionGroup, type OfferedAction } from "../authoritative/gameplay";
import type { Snapshot } from "../authoritative/state";
import type { ControlView } from "./control";

function targetsActor(action: OfferedAction, actorId: string): boolean {
  const intent = action.intent;
  if (intent?.kind === "physical_attack") return intent.target_actor_id === actorId;
  if (intent?.kind === "cast_spell" || intent?.kind === "cast_warmed_spell") {
    const target = intent.target as { kind: string; actor_id?: string } | null;
    return target?.kind === "actor" && target.actor_id === actorId;
  }
  return false;
}

/** Provider and combat target identities come from the server, never names or art. */
export function actorActionGroups(snapshot: Snapshot, actorId: string): ActionGroup[] {
  const frame = snapshot.envelope.frame;
  if (!frame.actors.some(actor => actor.actor_id === actorId)) return [];
  // Explicit traversal is only ever the observed character's own move. The
  // server supplies the intent; the menu never derives stairs from terrain.
  const observer = actorId === frame.observer_actor_id;
  const keys = new Set(frame.services_here.filter(service => service.actor_id === actorId)
    .map(service => `service:${service.service_id}`));
  keys.add(`npc:${actorId}`);
  return actionGroups(frame).flatMap(group => {
    if (keys.has(group.key)) return [group];
    if (group.key !== "character") return [];
    const actions = group.actions.filter(action => targetsActor(action, actorId)
      || (observer && action.enabled && action.intent?.kind === "traverse"));
    return actions.length ? [{ ...group, actions }] : [];
  });
}

/** Opening an actor only reveals options. Dispatch re-resolves the current offer. */
export class ActorInteraction {
  private actorId: string | null = null;
  private generation: number | null = null;
  private readonly root = document.createElement("dialog");
  private readonly title = document.createElement("h2");
  private readonly content = document.createElement("div");
  private readonly close = document.createElement("button");
  private readonly buttons: HTMLButtonElement[] = [];
  constructor(private readonly dispatch: (generation: number, group: string, action: string) => boolean) {
    this.root.className = "resident-dialog";
    this.title.id = "resident-title"; this.root.setAttribute("aria-labelledby", this.title.id);
    this.close.type = "button"; this.close.textContent = "Close";
    this.close.onclick = () => this.dismiss();
    this.root.addEventListener("close", () => { this.actorId = null; });
    this.root.append(this.title, this.content, this.close); document.body.append(this.root);
  }

  open(actorId: string, view: ControlView): void {
    if (!view.snapshot?.envelope.frame.actors.some(actor => actor.actor_id === actorId)) return;
    this.actorId = actorId; this.generation = null; this.present(view);
    if (this.actorId && !this.root.open) this.root.show();
  }

  private dismiss(): void { this.actorId = null; this.root.close(); }

  present(view: ControlView): void {
    if (!this.actorId) return;
    const snapshot = view.snapshot;
    const actor = snapshot?.envelope.frame.actors.find(row => row.actor_id === this.actorId);
    if (!snapshot || !actor || view.phase !== "playing") { this.dismiss(); return; }
    if (snapshot.generation !== this.generation) {
      const focused = this.content.contains(document.activeElement)
        ? (document.activeElement as HTMLElement).dataset.action : undefined;
      this.generation = snapshot.generation; this.title.textContent = actor.name;
      this.content.replaceChildren(); this.buttons.length = 0;
      const groups = actorActionGroups(snapshot, this.actorId);
      if (!groups.length) {
        const hint = document.createElement("p");
        hint.textContent = "No actions are currently offered for this character. Services require sharing their square.";
        this.content.append(hint);
      }
      for (const group of groups) {
        for (const fact of group.facts) {
          const p = document.createElement("p"); p.textContent = fact; this.content.append(p);
        }
        for (const action of group.actions) {
          const button = document.createElement("button"); button.type = "button";
          const listing = snapshot.envelope.frame.services_here
            .filter(service => group.key === `service:${service.service_id}`)
            .flatMap(service => service.capabilities.flatMap(capability =>
              capability.kind === "merchant" ? capability.listings : []))
            .find(row => row.purchase.id === action.id);
          button.textContent = listing
            ? `Buy ${listing.item.name} × ${listing.item.quantity} — ${listing.price_gold} gold`
            : action.label;
          button.dataset.action = `${group.key}/${action.id}`;
          const unavailable = !action.enabled || !action.intent;
          button.dataset.unavailable = String(unavailable);
          if (action.blocked_reason) button.title = action.blocked_reason.replaceAll("_", " ");
          button.onclick = () => { this.dispatch(snapshot.generation, group.key, action.id); };
          this.content.append(button); this.buttons.push(button);
          if (action.blocked_reason) {
            const reason = document.createElement("small");
            reason.textContent = action.blocked_reason.replaceAll("_", " "); this.content.append(reason);
          }
        }
      }
      this.buttons.find(button => button.dataset.action === focused)?.focus({ preventScroll: true });
    }
    for (const button of this.buttons) button.disabled = view.busy || view.pending
      || !snapshot.envelope.frame.can_act || button.dataset.unavailable === "true";
  }
}
