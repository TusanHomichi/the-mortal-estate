import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import net from "node:net";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const proofDirectory = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(proofDirectory, "..");
const captureRoot = "/data/dev/home/tme-visual-lab/web-feel-v26/walk-captures";
const assetRoot = "/data/dev/home/tme-visual-lab/engine-feel-v25/assets";

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  assert(address !== null && typeof address === "object");
  const port = address.port;
  await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  return port;
}

async function waitForVite(url, server, output) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) throw new Error(`Vite exited before serving the proof:\n${output()}`);
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // The selected port is not accepting connections yet.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Vite did not serve the proof in time:\n${output()}`);
}

async function stopServer(server) {
  if (server.exitCode !== null) return;
  server.kill("SIGTERM");
  const stopped = await Promise.race([
    new Promise((resolve) => server.once("exit", () => resolve(true))),
    new Promise((resolve) => setTimeout(() => resolve(false), 2_000)),
  ]);
  if (!stopped && server.exitCode === null) server.kill("SIGKILL");
}

function parseCell(value) {
  assert.match(value, /^\d+,\d+$/);
  return value.split(",").map(Number);
}

const port = await freePort();
const baseUrl = `http://127.0.0.1:${port}/`;
let serverOutput = "";
const server = spawn(
  process.execPath,
  [path.join(webRoot, "node_modules/vite/bin/vite.js"), "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
  {
    cwd: webRoot,
    env: { ...process.env, TME_FEEL_ASSETS: assetRoot },
    stdio: ["ignore", "pipe", "pipe"],
  },
);
server.stdout.on("data", (chunk) => { serverOutput += chunk.toString(); });
server.stderr.on("data", (chunk) => { serverOutput += chunk.toString(); });

let browser;
try {
  await waitForVite(baseUrl, server, () => serverOutput);
  await mkdir(captureRoot, { recursive: true });
  browser = await chromium.launch({ executablePath: chromium.executablePath(), headless: true });
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const consoleErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(`console: ${message.text()}`);
  });
  page.on("pageerror", (error) => consoleErrors.push(`page: ${error.message}`));

  await page.goto(`${baseUrl}?preset=night`, { waitUntil: "networkidle" });
  await page.waitForFunction(() => document.body.dataset.sceneReady === "true");
  await page.waitForFunction(() => document.querySelector("#feel-stage")?.dataset.walkState === "idle");
  assert.equal(await page.locator("#feel-stage").getAttribute("data-caretaker-cell"), "5,5");

  const [target, repreviewTarget] = await page.evaluate(async () => {
    const { createFeelCamera } = await import("/src/camera.ts");
    const camera = createFeelCamera(innerWidth, innerHeight, { i: 12, j: 9 });
    return [[2, 5], [4, 3]].map(([i, j]) => {
      const projected = camera.position.clone().set(i, 0, j).project(camera);
      return {
        x: (projected.x + 1) * innerWidth * 0.5,
        y: (1 - projected.y) * innerHeight * 0.5,
      };
    });
  });

  await page.mouse.click(target.x, target.y);
  assert.equal(await page.locator("#feel-stage").getAttribute("data-walk-state"), "preview");
  await page.screenshot({ path: path.join(captureRoot, "walk-preview.png") });

  const start = parseCell((await page.locator("#feel-stage").getAttribute("data-caretaker-cell")) ?? "");
  await page.mouse.dblclick(target.x, target.y);
  await page.waitForFunction(() => document.querySelector("#feel-stage")?.dataset.walkState === "committed");
  const committedAt = performance.now();
  await page.waitForTimeout(800);
  await page.mouse.click(repreviewTarget.x, repreviewTarget.y);
  assert.equal(await page.locator("#feel-stage").getAttribute("data-walk-state"), "committed");
  await page.screenshot({ path: path.join(captureRoot, "walk-repreview.png") });

  await page.waitForTimeout(Math.max(0, 3_400 - (performance.now() - committedAt)));
  const afterOneBeat = parseCell((await page.locator("#feel-stage").getAttribute("data-caretaker-cell")) ?? "");
  assert.equal(Math.max(Math.abs(afterOneBeat[0] - start[0]), Math.abs(afterOneBeat[1] - start[1])), 1);
  await page.screenshot({ path: path.join(captureRoot, "walk-step.png") });

  await page.waitForFunction(
    () => document.querySelector("#feel-stage")?.dataset.caretakerCell === "2,5",
    undefined,
    { timeout: 15_000 },
  );
  assert.equal(await page.locator("#feel-stage").getAttribute("data-walk-state"), "preview");
  await page.keyboard.press("Escape");
  await page.waitForFunction(() => document.querySelector("#feel-stage")?.dataset.walkState === "idle");
  assert.equal(await page.locator("#feel-stage").getAttribute("data-caretaker-cell"), "2,5");
  assert.deepEqual(consoleErrors, []);
  console.log(`PASS walk proof: ${captureRoot}`);
} finally {
  await browser?.close();
  await stopServer(server);
}
