import type { Attributes, ControlView, CreationDraft, CreationOption } from "./control";

const attributes: (keyof Attributes)[] = ["strength", "dexterity", "constitution", "intelligence", "wisdom", "charisma"];

export class CreationPanel {
  private options: readonly CreationOption[] = [];
  private readonly profile = document.createElement("select");
  private readonly name = document.createElement("input");
  private readonly fields = new Map<keyof Attributes, HTMLInputElement>();
  private readonly budget = document.createElement("p");
  private readonly submit = document.createElement("button");
  constructor(private readonly root: HTMLFormElement, create: (draft: CreationDraft) => Promise<void>) {
    const label = (text: string, control: HTMLElement) => { const row = document.createElement("label"); row.append(text, control); root.append(row); };
    this.profile.id = "creation-profile"; label("Class and origin", this.profile);
    this.name.id = "creation-name"; this.name.required = true; this.name.maxLength = 64; label("Character name", this.name);
    for (const attribute of attributes) {
      const input = document.createElement("input"); input.type = "number"; input.step = "1"; input.required = true;
      input.id = `creation-${attribute}`; this.fields.set(attribute, input);
      label(attribute[0]!.toUpperCase() + attribute.slice(1), input);
      input.oninput = () => this.updateBudget();
    }
    this.submit.type = "submit"; this.submit.textContent = "Create character"; root.append(this.budget, this.submit);
    this.profile.onchange = () => this.populate();
    root.onsubmit = event => {
      event.preventDefault();
      const values = Object.fromEntries(attributes.map(key => [key, Number(this.fields.get(key)!.value)])) as unknown as Attributes;
      void create({ profile_id: this.profile.value, display_name: this.name.value, attributes: values }).catch(() => {});
    };
  }
  present(view: ControlView): void {
    this.root.hidden = view.phase !== "selecting" || view.creationOptions.length === 0;
    if (this.options !== view.creationOptions) {
      this.options = view.creationOptions;
      this.profile.replaceChildren(...this.options.map(option => {
        const row = document.createElement("option"); row.value = option.profile_id;
        row.textContent = `${option.class_name} · ${option.nationality}`; return row;
      }));
      this.populate();
    }
    for (const control of [this.profile, this.name, ...this.fields.values(), this.submit]) control.disabled = view.busy;
    if (view.phase === "signed_out") { this.name.value = ""; for (const field of this.fields.values()) field.value = ""; }
  }
  private populate(): void {
    const option = this.options.find(row => row.profile_id === this.profile.value);
    if (!option) return;
    for (const key of attributes) {
      const field = this.fields.get(key)!;
      field.min = String(option.minimum[key]); field.max = String(option.maximum[key]); field.value = String(option.suggested[key]);
    }
    this.updateBudget();
  }
  private updateBudget(): void {
    const option = this.options.find(row => row.profile_id === this.profile.value);
    if (!option) return;
    const spent = attributes.reduce((sum, key) => sum + Number(this.fields.get(key)!.value) - option.minimum[key], 0);
    this.budget.textContent = `${spent} of ${option.attribute_points} attribute points allocated.`;
  }
}
