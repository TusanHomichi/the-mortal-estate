// The one way a proof or a capture serves the feel scene: Vite on a free
// loopback port, the candidate packet from the environment, the server's
// whole process group stopped afterwards. Nothing here asserts anything
// about the scene; it only gets a real tab in front of it.
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium, firefox } from "playwright";

export const webRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

/** Exit 3 — the verification runner's INCOMPLETE — with the reason on stderr. */
export function refuseUnavailable(reason) {
  console.error(`UNAVAILABLE: ${reason}`);
  process.exit(3);
}

/**
 * The same refusal, thrown: for a capability found missing after a server is
 * already up, so the caller's cleanup runs before the process reports 3.
 */
export class ProofUnavailable extends Error {
  exitCode = 3;
}

/** Report a thrown refusal the way refuseUnavailable would, without exiting. */
export function reportUnavailable(error) {
  console.error(`UNAVAILABLE: ${error.message}`);
  process.exitCode = 3;
}

/** The candidate packet directory, or a refusal: a tracked path is never a default. */
export function requirePacket() {
  const packet = process.env.TME_FEEL_ASSETS?.trim() ?? "";
  if (packet === "") {
    refuseUnavailable("TME_FEEL_ASSETS is not set; it must name the candidate packet directory");
  }
  return path.resolve(packet);
}

/**
 * The engines every real-tab proof runs in (owner ruling, 2026-09-03): a
 * picture judged in one browser is judged in both. `TME_PROOF_BROWSER` narrows
 * a run to `chromium` or `firefox` for a single-engine look; the default is
 * both, and an engine whose binary Playwright has not installed refuses the
 * whole run with exit 3 rather than passing on the other one.
 */
export const PROOF_ENGINES = { chromium, firefox };

export function proofBrowsers() {
  const requested = process.env.TME_PROOF_BROWSER?.trim() || "all";
  const names = requested === "all" ? Object.keys(PROOF_ENGINES) : [requested];
  const selected = [];
  for (const name of names) {
    const engine = PROOF_ENGINES[name];
    if (engine === undefined) {
      console.error(`TME_PROOF_BROWSER names an unknown engine: ${name}`);
      process.exit(2);
    }
    const executablePath = engine.executablePath();
    if (!existsSync(executablePath)) {
      refuseUnavailable(`Playwright has no ${name} at ${executablePath}; run: npx playwright install ${name}`);
    }
    selected.push({ name, engine, executablePath });
  }
  return selected;
}

/**
 * Launch one proof engine the way it can actually render the scene. Chromium
 * renders WebGL2 headless. Firefox has no GL context headless at all — probed
 * 2026-09-03: no preference enables it — so it runs headed with software GL
 * on a display: `DISPLAY` if the environment has one, otherwise an Xvfb this
 * helper starts and stops. No display and no Xvfb is a refusal, not a pass.
 */
export async function launchProofBrowser({ name, engine, executablePath }) {
  if (name === "chromium") {
    const browser = await engine.launch({ executablePath, headless: true });
    return { browser, stop: async () => { await browser.close(); } };
  }
  let display = process.env.DISPLAY?.trim() || "";
  let xvfb = null;
  if (display === "") {
    let xvfbPath = "";
    try { xvfbPath = execFileSync("which", ["Xvfb"], { encoding: "utf8" }).trim(); } catch { /* absent */ }
    if (xvfbPath === "") throw new ProofUnavailable("Firefox needs a display for WebGL2: set DISPLAY or install Xvfb");
    const number = 90 + Math.floor(Math.random() * 900);
    display = `:${number}`;
    xvfb = spawn(xvfbPath, [display, "-screen", "0", "1600x1000x24", "-nolisten", "tcp"], { stdio: "ignore" });
    await new Promise((resolve) => setTimeout(resolve, 600));
    if (xvfb.exitCode !== null) throw new ProofUnavailable(`Xvfb ${display} exited with ${xvfb.exitCode}`);
  }
  const browser = await engine.launch({
    executablePath,
    headless: false,
    firefoxUserPrefs: { "webgl.force-enabled": true, "webgl.forbid-software": false },
    env: { ...process.env, DISPLAY: display, LIBGL_ALWAYS_SOFTWARE: "1" },
  });
  return {
    browser,
    stop: async () => {
      await browser.close();
      if (xvfb !== null && xvfb.exitCode === null) xvfb.kill("SIGTERM");
    },
  };
}

export async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  if (address === null || typeof address !== "object") throw new Error("no loopback port");
  const { port } = address;
  await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  return port;
}

async function waitForVite(url, server, output) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`Vite exited before serving:\n${output()}`);
    }
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The selected loopback port is not accepting connections yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Vite did not serve in time:\n${output()}`);
}

/** Stop the server's whole process group: Vite's own children outlive a plain kill. */
async function stopServer(server) {
  if (server.exitCode !== null) return;
  const signal = (name) => {
    try {
      process.kill(-server.pid, name);
    } catch {
      server.kill(name);
    }
  };
  signal("SIGTERM");
  const stopped = await Promise.race([
    new Promise((resolve) => server.once("exit", () => resolve(true))),
    new Promise((resolve) => setTimeout(() => resolve(false), 2_000)),
  ]);
  if (!stopped && server.exitCode === null) signal("SIGKILL");
}

/** Serve the scene from `packet`; resolve once a tab can load it. */
export async function startVite(packet) {
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}/`;
  let serverOutput = "";
  const server = spawn(
    process.execPath,
    [path.join(webRoot, "node_modules/vite/bin/vite.js"), "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: webRoot,
      env: { ...process.env, TME_FEEL_ASSETS: packet },
      stdio: ["ignore", "pipe", "pipe"],
      detached: true,
    },
  );
  server.stdout.on("data", (chunk) => { serverOutput += chunk.toString(); });
  server.stderr.on("data", (chunk) => { serverOutput += chunk.toString(); });
  const output = () => serverOutput;
  try {
    await waitForVite(baseUrl, server, output);
  } catch (error) {
    await stopServer(server);
    throw error;
  }
  return { baseUrl, server, output, stop: () => stopServer(server) };
}
