import type { ControlView } from "./control";
import { classEmblem } from "./creationTheme";

/** Menu selection is presentation state; admission and creation stay in PlayControl. */
export class EntryShell {
  private selected = "";
  private rosterIdentity = "";
  private created: string | null = null;
  private lastScreen = "";
  private readonly root = document.getElementById("entry")!;
  private readonly roster = document.getElementById("character") as HTMLFieldSetElement;
  private readonly title = document.getElementById("entry-title")!;
  get characterId(): string { return this.selected; }
  present(view: ControlView): void {
    const screen = view.phase === "selecting" ? view.creationOptions.length ? "creation" : "roster" : view.phase;
    document.body.dataset.entryScreen = screen;
    this.root.hidden = view.phase === "playing";
    document.getElementById("roster-sheet")!.hidden = screen !== "roster";
    document.getElementById("entry-status")!.hidden = !["connecting", "disconnected"].includes(screen);
    this.title.textContent = screen === "creation" ? "Create a character" : screen === "roster" ? "Choose your character" : screen === "connecting" ? "Entering the world" : screen === "disconnected" ? "Connection lost" : "The Mortal Estate";
    const identity = JSON.stringify(view.characters);
    if (identity !== this.rosterIdentity) {
      this.rosterIdentity = identity;
      if (!view.characters.some(row => row.character_id === this.selected)) this.selected = view.characters[0]?.character_id ?? "";
      this.roster.replaceChildren();
      for (const character of view.characters) {
        const row = document.createElement("label"); row.className = "roster-choice";
        const radio = document.createElement("input"); radio.type = "radio"; radio.name = "character"; radio.value = character.character_id; radio.setAttribute("aria-label", character.display_name);
        radio.onchange = () => { this.selected = radio.value; };
        const name = document.createElement("span"); name.textContent = character.display_name;
        const detail = document.createElement("small"); detail.textContent = `Character ${character.slot}`;
        const text = document.createElement("span"); text.append(name, detail); row.append(radio, classEmblem("lineage"), text); this.roster.append(row);
      }
    }
    if (view.createdCharacterId !== this.created) {
      this.created = view.createdCharacterId; if (this.created) this.selected = this.created;
    }
    this.roster.disabled = view.busy;
    this.roster.querySelectorAll<HTMLInputElement>("input").forEach(radio => { radio.checked = radio.value === this.selected; });
    document.getElementById("empty-roster")!.hidden = view.characters.length !== 0;
    if (screen !== this.lastScreen) {
      if (screen === "creation") document.querySelector<HTMLInputElement>("#creation-profile input:checked")?.focus();
      if (screen === "roster") (this.roster.querySelector<HTMLInputElement>("input:checked") ?? document.getElementById("open-creation"))?.focus();
      this.lastScreen = screen;
    }
  }
}
