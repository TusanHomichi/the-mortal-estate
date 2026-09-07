import "./style.css";
import { WireCodec } from "../authoritative/codec";
import { AuthoritativeRenderer } from "../authoritative/renderer";
import { PlayControl, type ControlView } from "./control";
import { ActorInteraction } from "./actorInteraction";
import { GameplayPanel } from "./gameplayPanel";
import { CreationPanel } from "./creationPanel";
import { actions, loadPreferences, savePreferences, type Action } from "./preferences";
import { PathControls } from "./pathControls";
import { walkCursorDataUris, WALK_CURSOR_HOTSPOT } from "../walk/cursors";

const element = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const canvas = element<HTMLCanvasElement>("world-canvas");
// A deployed artwork build cannot fall back to diagnostics through its URL.
const rendered = import.meta.env.TME_PLAY_PRESENTATION === "first-expedition" ||
  new URLSearchParams(location.search).get("study") === "first-expedition";
const view = rendered
  ? await import("./studyRenderer").then(module => module.ExpeditionStudyRenderer.create(canvas, 768, 512))
  : new AuthoritativeRenderer(canvas, 768, 512);
const directions: Record<string, string> = { "play.north": "north", "play.east": "east", "play.south": "south", "play.west": "west" };
const buttons = [...document.querySelectorAll<HTMLButtonElement>("[data-action]")];
let preferences = loadPreferences();
let control: PlayControl;
const cursors = walkCursorDataUris();
const walk = new PathControls({ previewPath: path => control.previewPath(path),
  cancelPathPreview: () => control?.cancelPathPreview(), command: intent => control.command(intent) }, state => {
  view.presentWalk(state);
  canvas.style.cursor = `url("${cursors[state.cursor]}") ${WALK_CURSOR_HOTSPOT.x} ${WALK_CURSOR_HOTSPOT.y}, ${state.cursor === "waiting" ? "wait" : "default"}`;
  canvas.dataset.walkState = state.route ? state.kind : "idle";
  canvas.dataset.walkRoute = JSON.stringify(state.route ?? []);
  canvas.dataset.walkCursor = state.cursor;
  const announcement = element("movement-announcement");
  const text = state.cursor === "waiting" ? "Waiting for server readiness." : state.cursor === "refused"
    ? "That square cannot be reached in this move." : state.route ? "Route planned. Click its endpoint again to move. Escape clears it." : "Click a square to plan your route.";
  if (announcement.textContent !== text) announcement.textContent = text;
});
const gameplay = new GameplayPanel(element("gameplay"), (generation, group, action, amount) => control.offeredAction(generation, group, action, amount));
const residentInteraction = new ActorInteraction((generation, group, action) => control.offeredAction(generation, group, action));
const creation = new CreationPanel(element<HTMLFormElement>("creation-form"), draft => control.createCharacter(draft));
let shownCreatedCharacter: string | null = null;
let shownGeneration: number | null = null;
let lastPhase = "";
let current: ControlView;
let arrival = performance.now();
function present(state: ControlView): void {
  current = state;
  gameplay.present(state);
  residentInteraction.present(state);
  creation.present(state);
  element("signin").hidden = state.phase !== "signed_out";
  element("selection").hidden = state.phase !== "selecting";
  element("world").hidden = state.phase === "signed_out" || state.phase === "selecting";
  element("session-actions").hidden = state.phase === "signed_out";
  element("connection").textContent = { signed_out: "Signed out", selecting: "Choose your character", connecting: "Connecting…", playing: "Connected", disconnected: "Disconnected — world authority cleared" }[state.phase];
  element("feedback").textContent = state.feedback;
  for (const id of ["login", "enter", "reconnect", "logout", "open-creation"]) element<HTMLButtonElement>(id).disabled = state.busy;
  element<HTMLButtonElement>("enter").disabled = state.busy || state.characters.length === 0;
  for (const button of buttons) button.disabled = state.busy || state.pending || state.phase !== "playing" || !state.snapshot?.envelope.frame.can_act;
  const select = element<HTMLSelectElement>("character");
  if ([...select.options].map(option => option.value).join() !== state.characters.map(row => row.character_id).join()) {
    select.replaceChildren(...state.characters.map(row => { const option = document.createElement("option"); option.value = row.character_id; option.textContent = row.display_name; return option; }));
  }
  if (state.createdCharacterId !== shownCreatedCharacter) {
    shownCreatedCharacter = state.createdCharacterId;
    if (shownCreatedCharacter) select.value = shownCreatedCharacter;
  }
  if (state.snapshot?.generation !== shownGeneration) {
    shownGeneration = state.snapshot?.generation ?? null; arrival = performance.now();
    if (state.snapshot) {
      try {
        view.present(state.snapshot);
        delete canvas.dataset.presentationError;
      } catch {
        view.clear();
        canvas.dataset.presentationError = "unavailable";
        element("feedback").textContent = "The presentation does not match this world, or could not be rendered.";
      }
      const frame = state.snapshot.envelope.frame, p = frame.observation_center.position;
      element("position").textContent = `Your square: ${p.x}, ${p.y} · ${frame.observation_center.level}`;
      element("occupants").replaceChildren(...frame.actors.map(actor => {
        const row = document.createElement("li"); row.textContent = `${actor.name}${actor.actor_id === frame.observer_actor_id ? " (you)" : ""} — ${actor.position.position.x}, ${actor.position.position.y}`; return row;
      }));
    } else { view.clear(); element("position").textContent = "Awaiting the server."; element("occupants").replaceChildren(); }
  }
  walk.present(state);
  document.body.dataset.phase = state.phase;
  // Sanitized authority facts support the installed UI proof and operator diagnosis.
  canvas.dataset.actor = state.snapshot?.envelope.frame.observer_actor_id ?? "";
  canvas.dataset.readyAt = state.snapshot?.envelope.frame.ready_at ?? "";
  canvas.dataset.logicalTime = state.snapshot?.envelope.frame.logical_time ?? "";
  canvas.dataset.canAct = String(state.snapshot?.envelope.frame.can_act ?? false);
  canvas.dataset.sequence = state.nextSequence;
  canvas.dataset.worldRevision = state.snapshot?.envelope.world_revision ?? "";
  canvas.dataset.pending = String(state.pending);
  if (lastPhase !== state.phase) {
    if (state.phase === "selecting") select.focus();
    if (state.phase === "playing") canvas.focus();
    if (state.phase === "signed_out") element("username").focus();
    lastPhase = state.phase;
  }
}
function act(action: Action): void {
  walk.cancel();
  control.command(action === "play.wait" ? { kind: "wait" } : { kind: "move_path", path: [directions[action]] });
}
for (const button of buttons) button.onclick = () => act(button.dataset.action as Action);
function pointer(event: MouseEvent) {
  const rect = canvas.getBoundingClientRect();
  return view.pointer((event.clientX - rect.left) * view.width / rect.width, (event.clientY - rect.top) * view.height / rect.height)?.coordinate ?? null;
}
canvas.addEventListener("click", event => { canvas.focus(); walk.click(pointer(event)); });
canvas.addEventListener("mousemove", event => walk.hoverAt(pointer(event)));
canvas.addEventListener("mouseleave", () => walk.hoverAt(null));
canvas.addEventListener("contextmenu", event => {
  event.preventDefault(); walk.cancel();
  if (!("pickActor" in view) || !current.snapshot || current.phase !== "playing") return;
  event.preventDefault();
  const rect = canvas.getBoundingClientRect();
  const actorId = view.pickActor((event.clientX - rect.left) * view.width / rect.width,
    (event.clientY - rect.top) * view.height / rect.height);
  if (actorId) residentInteraction.open(actorId, current);
});
document.addEventListener("keydown", event => {
  if (event.code === "Escape") { walk.cancel(); return; }
  if (event.repeat || event.altKey || event.ctrlKey || event.metaKey || event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement || (event.target instanceof HTMLButtonElement && event.code === "Space")) return;
  const action = actions.find(action => preferences.bindings[action] === event.code);
  if (action && current?.phase === "playing") { event.preventDefault(); act(action); }
});
function settings(): void {
  document.documentElement.style.fontSize = `${preferences.textScale}%`;
  element<HTMLSelectElement>("text-scale").value = String(preferences.textScale);
}
element<HTMLSelectElement>("text-scale").onchange = event => { preferences.textScale = Number((event.target as HTMLSelectElement).value); savePreferences(preferences); settings(); };
for (const action of actions) {
  const label = document.createElement("label"); label.textContent = action.replace("play.", "");
  const input = document.createElement("input"); input.value = preferences.bindings[action]; input.readOnly = true; input.setAttribute("aria-label", `${label.textContent} key`);
  input.onkeydown = event => {
    if (event.code === "Tab") return;
    event.preventDefault();
    if (!/^(Arrow(Up|Right|Down|Left)|Space|Key[A-Z]|Digit[0-9])$/.test(event.code) || actions.some(other => other !== action && preferences.bindings[other] === event.code)) {
      element("settings-status").textContent = "Choose an unused arrow, letter, number, or Space key."; return;
    }
    preferences.bindings[action] = event.code; input.value = event.code; savePreferences(preferences); element("settings-status").textContent = "Control preference saved.";
  };
  label.append(input); element("bindings").append(label);
}
settings();
const codecResponse = await fetch("/codec.wasm", { credentials: "omit", cache: "no-store" });
if (!codecResponse.ok) throw new Error("Protocol codec unavailable");
control = new PlayControl(await WireCodec.create(await codecResponse.arrayBuffer()), location.origin, present);
element<HTMLFormElement>("login-form").onsubmit = event => {
  event.preventDefault(); const password = element<HTMLInputElement>("password");
  const value = password.value; password.value = "";
  void control.login(element<HTMLInputElement>("username").value, value).catch(() => {});
};
element("enter").onclick = () => { void control.select(element<HTMLSelectElement>("character").value).catch(() => {}); };
element("open-creation").onclick = () => { void control.openCreation().catch(() => {}); };
element("reconnect").onclick = () => { void control.reconnect().catch(() => {}); };
element("logout").onclick = () => { void control.logout().catch(() => {}); };
function progress(): void {
  const frame = current?.snapshot?.envelope.frame;
  const remaining = frame ? BigInt(frame.ready_at) - BigInt(frame.logical_time) : 0n;
  const elapsed = Math.max(0, performance.now() - arrival);
  // Cosmetic interpolation cannot grant readiness. No number conversion of IDs/counters.
  const bounded = remaining > 60_000n ? 60_000 : remaining > 0n ? Number(remaining) : 0;
  const fraction = frame?.can_act ? 1 : bounded ? Math.min(1, elapsed / bounded) : 0;
  element<HTMLProgressElement>("cooldown").value = fraction;
  element("readiness").textContent = !frame ? "Awaiting the server." : current.pending ? "Awaiting action result…" : frame.can_act ? "Ready" : fraction === 1 ? "Awaiting readiness confirmation…" : "Recovering…";
  requestAnimationFrame(progress);
}
progress();
window.addEventListener("pagehide", () => { control.dispose(); view.dispose(); });
element<HTMLFieldSetElement>("login-controls").disabled = false;
document.body.dataset.playReady = "true";
