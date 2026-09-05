export const actions = ["play.north", "play.east", "play.south", "play.west", "play.wait"] as const;
export type Action = typeof actions[number];
export interface Preferences { version: 1; textScale: number; bindings: Record<Action, string> }
export const defaults = (): Preferences => ({ version: 1, textScale: 100, bindings: {
  "play.north": "ArrowUp", "play.east": "ArrowRight", "play.south": "ArrowDown", "play.west": "ArrowLeft", "play.wait": "Space",
} });
const key = "tme.play.preferences";
export function loadPreferences(): Preferences {
  try {
    const value = JSON.parse(localStorage.getItem(key) ?? "null");
    if (!value || value.version !== 1 || Object.keys(value).sort().join() !== "bindings,textScale,version"
      || ![100, 150, 200].includes(value.textScale) || !value.bindings
      || Object.keys(value.bindings).sort().join() !== [...actions].sort().join()
      || actions.some(action => !/^(Arrow(Up|Right|Down|Left)|Space|Key[A-Z]|Digit[0-9])$/.test(value.bindings[action]))
      || new Set(Object.values(value.bindings)).size !== actions.length) return defaults();
    return value;
  } catch { return defaults(); }
}
export function savePreferences(value: Preferences): void {
  localStorage.setItem(key, JSON.stringify(value));
}
