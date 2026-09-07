import { goldAmountField, type ActionGroup } from "../authoritative/gameplay";

const node = <K extends keyof HTMLElementTagNameMap>(tag: K, text = "") => {
  const element = document.createElement(tag); element.textContent = text; return element;
};

/** One retained group; callbacks resolve its current offer, never a captured frame. */
export class GameplayGroup {
  readonly section = node("details");
  private readonly title = node("summary");
  private readonly facts = node("div");
  private readonly select = node("select");
  private readonly amountLabel = node("label", "Gold amount");
  private readonly amount = node("input");
  private readonly button = node("button", "Perform selected action");
  private readonly empty = node("p", "No actions offered here.");
  private group: ActionGroup | null = null;
  private locked = true;
  private optionsSignature = "";

  constructor(key: string, dispatch: (action: string, amount?: string) => boolean) {
    this.section.dataset.group = key; this.section.open = key.startsWith("service:");
    this.select.dataset.group = key;
    this.amount.inputMode = "numeric"; this.amount.maxLength = 19;
    this.button.type = "button";
    this.amountLabel.append(this.amount);
    this.section.append(this.title, this.facts, this.select, this.amountLabel, this.button, this.empty);
    this.button.onclick = () => {
      if (this.locked || !this.section.isConnected || !this.offered()) return;
      const accepted = dispatch(this.select.value,
        !this.amountLabel.hidden && this.amount.value !== "" ? this.amount.value : undefined);
      this.amount.setCustomValidity(accepted ? "" : "Action unavailable or invalid amount. Choose again using the latest state.");
      if (!accepted && !this.amountLabel.hidden) this.amount.reportValidity();
    };
    this.amount.oninput = () => this.amount.setCustomValidity("");
    this.select.onchange = () => { this.amount.value = ""; this.amount.setCustomValidity(""); this.refresh(); };
  }

  private offered() {
    const matches = this.group?.actions.filter(action => action.id === this.select.value) ?? [];
    return matches.length === 1 && matches[0]!.enabled ? matches[0]!.intent : null;
  }

  present(group: ActionGroup): void {
    this.group = group;
    if (this.title.textContent !== group.title) this.title.textContent = group.title;
    this.facts.replaceChildren(...group.facts.map(fact => node("p", fact)));
    this.select.setAttribute("aria-label", `${group.title} action`);
    this.amount.setAttribute("aria-label", `${group.title} gold amount`);
    const signature = JSON.stringify(group.actions);
    if (signature !== this.optionsSignature) {
      this.optionsSignature = signature;
      const selected = this.select.value;
      const placeholder = node("option", "Choose an action…"); placeholder.value = "";
      this.select.replaceChildren(placeholder, ...group.actions.map(action => {
        const option = node("option", action.label + (action.blocked_reason ? ` — ${action.blocked_reason.replaceAll("_", " ")}` : ""));
        option.value = action.id; option.disabled = !action.enabled || !action.intent; return option;
      }));
      this.select.value = group.actions.some(action => action.id === selected) ? selected : "";
      if (!this.select.value) { this.amount.value = ""; this.amount.setCustomValidity(""); }
    }
    this.select.hidden = this.button.hidden = group.actions.length === 0;
    this.empty.hidden = group.actions.length !== 0;
    this.refresh();
  }

  lock(locked: boolean): void { this.locked = locked; this.refresh(); }
  retire(): void { this.group = null; this.lock(true); this.section.remove(); }

  private refresh(): void {
    const intent = this.offered(), field = goldAmountField(intent);
    this.amountLabel.hidden = !field;
    this.amount.placeholder = field === "quantity" ? "All available gold" : "";
    if (!this.amount.value && field && field !== "quantity") this.amount.value = String(intent![field]);
    this.select.disabled = this.amount.disabled = this.locked;
    this.button.disabled = this.locked || !intent;
  }
}
