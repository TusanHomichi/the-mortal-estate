// One roster for browser launchers, capability probes and Python producers.
import { readFileSync, existsSync } from "node:fs";
import { chromium, firefox, webkit } from "playwright";

export const ENGINE_NAMES = Object.freeze(JSON.parse(readFileSync(new URL("./engines.json", import.meta.url), "utf8")));
const available = { chromium, firefox, webkit };
if (!ENGINE_NAMES.length || new Set(ENGINE_NAMES).size !== ENGINE_NAMES.length || ENGINE_NAMES.some(name => !available[name])) {
  throw new Error("invalid browser proof roster");
}
export const PROOF_ENGINES = Object.freeze(Object.fromEntries(ENGINE_NAMES.map(name => [name, available[name]])));

export function resolveProofBrowsers(requested = "all", engines = PROOF_ENGINES, exists = existsSync) {
  return (requested === "all" ? Object.keys(engines) : [requested]).map(name => {
    const engine = engines[name];
    if (!engine) throw new Error(`TME_PROOF_BROWSER names an unknown engine: ${name}`);
    const executablePath = engine.executablePath();
    if (!exists(executablePath)) throw new Error(`Playwright has no ${name} at ${executablePath}; run: npx playwright install ${name}`);
    return { name, engine, executablePath };
  });
}
