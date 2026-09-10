import type { Attributes, ControlView, CreationDraft, CreationOption } from "./control";
import { allocationState, creationAttributes } from "./creationAllocation";
import { classEmblem, creationTheme } from "./creationTheme";

export class CreationPanel {
  private options: readonly CreationOption[] = [];
  private selected = "";
  private values: Attributes = { strength: 0, dexterity: 0, constitution: 0, intelligence: 0, wisdom: 0, charisma: 0 };
  private readonly classes = document.createElement("fieldset");
  private readonly name = document.createElement("input");
  private readonly portrait = document.createElement("div");
  private readonly heading = document.createElement("h2");
  private readonly description = document.createElement("p");
  private readonly fields = new Map<keyof Attributes, { value: HTMLOutputElement; bounds: HTMLElement; down: HTMLButtonElement; up: HTMLButtonElement; pips: HTMLElement }>();
  private readonly budget = document.createElement("output");
  private readonly submit = document.createElement("button");
  private readonly cancel: HTMLButtonElement;
  private readonly reset: HTMLButtonElement;
  private busy = false;
  private retry = false;
  constructor(private readonly root: HTMLFormElement, create: (draft: CreationDraft) => Promise<void>, cancel: () => void) {
    root.className = "creation-sheet";
    this.classes.id = "creation-profile"; this.classes.className = "class-list";
    const legend = document.createElement("legend"); legend.textContent = "Choose your calling"; this.classes.append(legend);
    const sheet = document.createElement("div"); sheet.className = "allocation-sheet";
    const introduction = document.createElement("div"); introduction.className = "class-introduction";
    this.portrait.className = "class-seal";
    const words = document.createElement("div"); words.append(this.heading, this.description); introduction.append(this.portrait, words);
    const nameLabel = document.createElement("label"); nameLabel.className = "character-name"; nameLabel.textContent = "Character name";
    this.name.id = "creation-name"; this.name.required = true; this.name.maxLength = 64; this.name.autocomplete = "off";
    this.name.placeholder = "Your name"; this.name.oninput = () => this.refresh(); nameLabel.append(this.name);
    const stats = document.createElement("div"); stats.className = "attribute-list";
    for (const attribute of creationAttributes) {
      const title = attribute[0]!.toUpperCase() + attribute.slice(1);
      const row = document.createElement("div"); row.className = "attribute-row";
      const label = document.createElement("label"); label.htmlFor = `creation-${attribute}`; label.textContent = title;
      const bounds = document.createElement("small"), pips = document.createElement("span"); pips.className = "attribute-pips"; pips.setAttribute("aria-hidden", "true");
      const value = document.createElement("output"); value.id = `creation-${attribute}`; value.setAttribute("aria-label", title);
      const down = this.button("−", () => this.change(attribute, -1)), up = this.button("+", () => this.change(attribute, 1));
      down.setAttribute("aria-label", `Decrease ${title}`); up.setAttribute("aria-label", `Increase ${title}`);
      row.append(label, bounds, pips, down, value, up); stats.append(row); this.fields.set(attribute, { value, bounds, down, up, pips });
    }
    const tools = document.createElement("div"); tools.className = "allocation-tools";
    this.reset = this.button("Reset points", () => this.populate()); tools.append(this.reset);
    sheet.append(introduction, nameLabel, stats, tools);
    const footer = document.createElement("footer"); footer.className = "creation-footer";
    this.budget.id = "creation-budget"; this.budget.setAttribute("aria-live", "polite");
    this.cancel = this.button("Back", cancel); this.submit.type = "submit"; this.submit.className = "primary-action"; this.submit.textContent = "Create character";
    footer.append(this.cancel, this.budget, this.submit); root.append(this.classes, sheet, footer);
    root.onsubmit = event => {
      event.preventDefault(); const option = this.option;
      if (this.busy || !option || !allocationState(option, this.values).valid || !this.name.value.trim()) return;
      void create({ profile_id: this.selected, display_name: this.name.value.trim(), attributes: { ...this.values } }).catch(() => {});
    };
  }
  private button(text: string, action: () => void): HTMLButtonElement {
    const button = document.createElement("button"); button.type = "button"; button.textContent = text; button.onclick = action; return button;
  }
  private get option() { return this.options.find(row => row.profile_id === this.selected); }
  present(view: ControlView): void {
    this.root.hidden = view.phase !== "selecting" || view.creationOptions.length === 0;
    this.busy = view.busy; this.retry = view.creationRetry !== null;
    if (this.options !== view.creationOptions) {
      this.options = view.creationOptions; this.selected = this.options[0]?.profile_id ?? "";
      this.classes.querySelectorAll("label").forEach(row => row.remove());
      for (const option of this.options) {
        const row = document.createElement("label"); row.className = "class-choice";
        const radio = document.createElement("input"); radio.type = "radio"; radio.name = "creation-class"; radio.value = option.profile_id;
        const text = document.createElement("span"); text.textContent = option.class_name;
        radio.onchange = () => { this.selected = radio.value; this.populate(); };
        row.append(radio, classEmblem(option.profile_id), text); this.classes.append(row);
      }
      this.populate();
    }
    if (view.creationRetry) {
      this.selected = view.creationRetry.profile_id; this.name.value = view.creationRetry.display_name; this.values = { ...view.creationRetry.attributes };
    }
    if (view.phase === "signed_out") this.name.value = "";
    this.refresh();
  }
  private populate(): void {
    if (!this.option) return;
    this.values = { ...this.option.minimum };
    this.portrait.replaceChildren(classEmblem(this.selected)); this.heading.textContent = this.option.class_name;
    this.description.textContent = creationTheme(this.selected).description; this.refresh();
  }
  private change(attribute: keyof Attributes, delta: number): void {
    const option = this.option; if (!option || this.busy || this.retry) return;
    const value = this.values[attribute] + delta;
    if (value < option.minimum[attribute] || value > option.maximum[attribute] || (delta > 0 && allocationState(option, this.values).remaining <= 0)) return;
    this.values[attribute] = value; this.refresh();
  }
  private refresh(): void {
    const option = this.option; if (!option) return;
    const state = allocationState(option, this.values), locked = this.busy || this.retry;
    this.classes.disabled = locked; this.name.disabled = locked; this.cancel.disabled = locked; this.reset.disabled = locked;
    this.classes.querySelectorAll<HTMLInputElement>("input").forEach(radio => { radio.checked = radio.value === this.selected; });
    for (const key of creationAttributes) {
      const field = this.fields.get(key)!; field.value.value = String(this.values[key]);
      field.bounds.textContent = `${option.minimum[key]}–${option.maximum[key]}`;
      field.down.disabled = locked || this.values[key] <= option.minimum[key]; field.up.disabled = locked || this.values[key] >= option.maximum[key] || state.remaining <= 0;
      field.pips.replaceChildren(...Array.from({ length: option.maximum[key] - option.minimum[key] }, (_, index) => {
        const pip = document.createElement("i"); pip.className = index < this.values[key] - option.minimum[key] ? "spent" : ""; return pip;
      }));
    }
    this.budget.value = this.retry ? "Confirm the previous creation to continue." : `${state.remaining} ${state.remaining === 1 ? "point" : "points"} remaining`;
    this.submit.disabled = this.busy || !state.valid || !this.name.value.trim();
    this.submit.textContent = this.busy ? "Creating…" : this.retry ? "Retry creation" : "Create character";
  }
}
