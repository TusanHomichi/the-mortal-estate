import "./style.css";
import { WireCodec } from "../authoritative/codec";
import { AuthoritativeRenderer } from "../authoritative/renderer";
import { PlayControl, type ControlView } from "./control";
import { actions, loadPreferences, savePreferences, type Action } from "./preferences";

const element = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;
const canvas = element<HTMLCanvasElement>("world-canvas");
const view = new AuthoritativeRenderer(canvas, 768, 512);
const directions: Record<string, string> = { "play.north": "north", "play.east": "east", "play.south": "south", "play.west": "west" };
const buttons = [...document.querySelectorAll<HTMLButtonElement>("[data-action]")];
let preferences = loadPreferences();
let control: PlayControl;
let shownGeneration: number | null = null;
let lastPhase = "";
let current: ControlView;
let arrival = performance.now();
function present(state: ControlView): void {
  current = state;
  element("signin").hidden = state.phase !== "signed_out";
  element("selection").hidden = state.phase !== "selecting";
  element("world").hidden = state.phase === "signed_out" || state.phase === "selecting";
  element("session-actions").hidden = state.phase === "signed_out";
  element("connection").textContent = { signed_out: "Signed out", selecting: "Choose your character", connecting: "Connecting…", playing: "Connected", disconnected: "Disconnected — world authority cleared" }[state.phase];
  element("feedback").textContent = state.feedback;
  for (const id of ["login", "enter", "reconnect", "logout"]) element<HTMLButtonElement>(id).disabled = state.busy;
  for (const button of buttons) button.disabled = state.busy || state.pending || state.phase !== "playing" || !state.snapshot?.envelope.frame.can_act;
  const select = element<HTMLSelectElement>("character");
  if ([...select.options].map(option => option.value).join() !== state.characters.map(row => row.character_id).join()) {
    select.replaceChildren(...state.characters.map(row => { const option = document.createElement("option"); option.value = row.character_id; option.textContent = row.display_name; return option; }));
  }
  if (state.snapshot?.generation !== shownGeneration) {
    shownGeneration = state.snapshot?.generation ?? null; arrival = performance.now();
    if (state.snapshot) {
      view.present(state.snapshot);
      const frame = state.snapshot.envelope.frame, p = frame.observation_center.position;
      element("position").textContent = `Your square: ${p.x}, ${p.y} · ${frame.observation_center.level}`;
      element("occupants").replaceChildren(...frame.actors.map(actor => {
        const row = document.createElement("li"); row.textContent = `${actor.name}${actor.actor_id === frame.observer_actor_id ? " (you)" : ""} — ${actor.position.position.x}, ${actor.position.position.y}`; return row;
      }));
    } else { view.clear(); element("position").textContent = "Awaiting the server."; element("occupants").replaceChildren(); }
  }
  document.body.dataset.phase = state.phase;
  // Sanitized authority facts support the installed UI proof and operator diagnosis.
  canvas.dataset.actor = state.snapshot?.envelope.frame.observer_actor_id ?? "";
  canvas.dataset.readyAt = state.snapshot?.envelope.frame.ready_at ?? "";
  canvas.dataset.logicalTime = state.snapshot?.envelope.frame.logical_time ?? "";
  canvas.dataset.canAct = String(state.snapshot?.envelope.frame.can_act ?? false);
  canvas.dataset.sequence = state.nextSequence;
  canvas.dataset.pending = String(state.pending);
  if (lastPhase !== state.phase) {
    if (state.phase === "selecting") select.focus();
    if (state.phase === "playing") canvas.focus();
    if (state.phase === "signed_out") element("username").focus();
    lastPhase = state.phase;
  }
}
function act(action: Action): void {
  control.command(action === "play.wait" ? { kind: "wait" } : { kind: "move_path", path: [directions[action]] });
}
for (const button of buttons) button.onclick = () => act(button.dataset.action as Action);
canvas.addEventListener("click", event => {
  const frame = current.snapshot?.envelope.frame; if (!frame) return;
  const rect = canvas.getBoundingClientRect();
  const target = view.pointer((event.clientX - rect.left) * view.width / rect.width, (event.clientY - rect.top) * view.height / rect.height);
  if (!target) return;
  const p = frame.observation_center.position, dx = target.coordinate.x - p.x, dy = target.coordinate.y - p.y;
  const action = dx === 0 && dy === -1 ? "play.north" : dx === 1 && dy === 0 ? "play.east" : dx === 0 && dy === 1 ? "play.south" : dx === -1 && dy === 0 ? "play.west" : null;
  if (action) act(action);
});
document.addEventListener("keydown", event => {
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
window.addEventListener("pagehide", () => control.dispose());
document.body.dataset.playReady = "true";
