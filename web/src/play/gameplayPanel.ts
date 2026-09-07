import { actionGroups } from "../authoritative/gameplay";
import type { ControlView } from "./control";
import { GameplayGroup } from "./gameplayGroup";

const node = <K extends keyof HTMLElementTagNameMap>(tag: K, text: string) => {
  const element = document.createElement(tag); element.textContent = text; return element;
};

/** Complete snapshots update retained controls without interrupting native gestures. */
export class GameplayPanel {
  private generation: number | null = null;
  private readonly groups = new Map<string, GameplayGroup>();
  private readonly sheet = node("section", "");
  private readonly inventory = node("section", "");
  constructor(private readonly root: HTMLElement,
    private readonly dispatch: (generation: number, group: string, action: string, amount?: string) => boolean) {
    this.sheet.setAttribute("aria-label", "Character sheet");
    this.inventory.setAttribute("aria-label", "Carried equipment");
  }

  present(view: ControlView): void {
    const snapshot = view.snapshot;
    const locked = view.phase !== "playing" || view.busy || view.pending || !snapshot?.envelope.frame.can_act;
    if ((snapshot?.generation ?? null) !== this.generation) {
      this.generation = snapshot?.generation ?? null;
      if (!snapshot) {
        for (const group of this.groups.values()) group.retire();
        this.groups.clear(); this.root.replaceChildren();
      } else {
        if (!this.sheet.isConnected) this.root.append(this.sheet, this.inventory);
        const frame = snapshot.envelope.frame, character = frame.character, resources = character.resources;
        const sheet = node("section", ""); sheet.setAttribute("aria-label", "Character sheet");
        sheet.append(node("h2", `${character.identity.display_class} · Level ${character.progression.level}`),
          node("p", `Health ${resources.hp}/${resources.max_hp} · Stamina ${resources.stamina}/${resources.max_stamina} · Mana ${resources.mp}/${resources.max_mp}`),
          node("p", `Experience: ${character.progression.experience}`));
        const skills = node("ul", "");
        for (const skill of character.skill_ledger) skills.append(node("li", `${skill.track_display ?? skill.track_id}: ${skill.level_title ?? `level ${skill.level}`} · rank ${skill.critique_rank}`));
        sheet.append(skills); this.sheet.replaceChildren(...sheet.childNodes);
        const inventory = node("section", ""); inventory.setAttribute("aria-label", "Carried equipment");
        inventory.append(node("h2", "Carried equipment"));
        const gold = frame.carried.gold;
        inventory.append(node("p", `Gold — sack: ${gold.sack}; left hand: ${gold.left_hand}; right hand: ${gold.right_hand}`));
        const items = node("ul", "");
        for (const entry of frame.carried.items) items.append(node("li", `${entry.position.replaceAll("_", " ")}: ${entry.item.name} × ${entry.item.quantity}`));
        inventory.append(items); this.inventory.replaceChildren(...inventory.childNodes);
        const offered = actionGroups(frame);
        const keys = new Set(offered.map(group => group.key));
        for (const [key, group] of this.groups) if (!keys.has(key)) {
          group.retire(); this.groups.delete(key);
        }
        for (const group of offered) {
          let control = this.groups.get(group.key);
          if (!control) {
            control = new GameplayGroup(group.key, (action, amount) => this.generation !== null &&
              this.dispatch(this.generation, group.key, action, amount));
            this.groups.set(group.key, control); this.root.append(control.section);
          }
          control.present(group);
        }
      }
    }
    for (const group of this.groups.values()) group.lock(locked);
  }
}
