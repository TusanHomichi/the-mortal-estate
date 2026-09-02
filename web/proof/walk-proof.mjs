import assert from "node:assert/strict";
import { copyFile, cp, mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import net from "node:net";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const proofDirectory = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(proofDirectory, "..");
const captureRoot = "/data/dev/home/tme-visual-lab/web-feel-v26/walk-captures";
const assetRoot = "/data/dev/home/tme-visual-lab/engine-feel-v25/assets";
const grownManifest =
  "/data/dev/home/tme-visual-lab/web-feel-v26/world/feel-manifest.grown.json";

async function freePort() {
  const server = net.createServer();
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const address = server.address();
  assert(address !== null && typeof address === "object");
  const port = address.port;
  await new Promise((resolve, reject) =>
    server.close((error) => (error ? reject(error) : resolve())),
  );
  return port;
}

async function waitForVite(url, server, output) {
  const deadline = Date.now() + 15_000;
  while (Date.now() < deadline) {
    if (server.exitCode !== null) {
      throw new Error(`Vite exited before serving the proof:\n${output()}`);
    }
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
  assert.match(value, /^-?\d+,-?\d+$/);
  const [i, j] = value.split(",").map(Number);
  return { i, j };
}

async function stageAttribute(page, name) {
  const value = await page.locator("#feel-stage").getAttribute(name);
  assert.notEqual(value, null, `${name} is absent`);
  return value;
}

function assertCustomCursorValue(value, fallback) {
  assert.match(value, /^url\(["']?data:image\/svg\+xml,/);
  assert.ok(value.endsWith(`, ${fallback}`), `cursor fallback is ${value}`);
}

async function assertCustomCursor(page, fallback) {
  const value = await page.locator("#feel-stage canvas").evaluate((canvas) => canvas.style.cursor);
  assertCustomCursorValue(value, fallback);
}

async function assertCommittedPresentation(page, pace) {
  const snapshot = await page.locator("#feel-stage").evaluate((stage) => ({
    state: stage.dataset.walkState,
    cursorKind: stage.dataset.walkCursor,
    pace: stage.dataset.walkPace,
    cursorValue: stage.querySelector("canvas")?.style.cursor ?? "",
  }));
  assert.equal(snapshot.state, "committed");
  assert.equal(snapshot.cursorKind, "waiting");
  assert.equal(snapshot.pace, pace);
  assertCustomCursorValue(snapshot.cursorValue, "wait");
}

async function assertCaretakerCentred(page) {
  const projection = await stageAttribute(page, "data-caretaker-projection");
  const [x, y] = projection.split(",").map(Number);
  assert.ok(Math.abs(x - 640) <= 1, `caretaker projects to x=${x}, not viewport centre`);
  assert.ok(Math.abs(y - 400) <= 1, `caretaker projects to y=${y}, not viewport centre`);
}

async function groundCellPoint(page, focus, target) {
  return page.evaluate(
    async ({ focusCell, targetCell }) => {
      const { createFeelCamera } = await import("/src/camera.ts");
      const camera = createFeelCamera(innerWidth, innerHeight, focusCell);
      const projected = camera.position.clone().set(targetCell.i, 0, targetCell.j).project(camera);
      return {
        x: (projected.x + 1) * innerWidth * 0.5,
        y: (1 - projected.y) * innerHeight * 0.5,
      };
    },
    { focusCell: focus, targetCell: target },
  );
}

async function visibleGroundCellPoint(page, focus, targets) {
  for (const target of targets) {
    const point = await groundCellPoint(page, focus, target);
    if (point.x > 1 && point.x < 1279 && point.y > 1 && point.y < 799) {
      return point;
    }
  }
  throw new Error("no five-square proof target projects inside the viewport");
}

async function captureThreeTimesRouteCrop(page, from, to, outputPath) {
  const [start, end] = await Promise.all([
    groundCellPoint(page, from, from),
    groundCellPoint(page, from, to),
  ]);
  const left = Math.max(0, Math.floor(Math.min(start.x, end.x) - 80));
  const top = Math.max(0, Math.floor(Math.min(start.y, end.y) - 190));
  const right = Math.min(1280, Math.ceil(Math.max(start.x, end.x) + 80));
  const bottom = Math.min(800, Math.ceil(Math.max(start.y, end.y) + 80));
  const source = await page.screenshot({
    clip: { x: left, y: top, width: right - left, height: bottom - top },
  });
  const scaledBase64 = await page.evaluate(
    async ({ encoded, scale }) => {
      const image = new Image();
      image.src = `data:image/png;base64,${encoded}`;
      await image.decode();
      const canvas = document.createElement("canvas");
      canvas.width = image.naturalWidth * scale;
      canvas.height = image.naturalHeight * scale;
      const context = canvas.getContext("2d");
      if (context === null) throw new Error("the proof could not create its review crop");
      context.imageSmoothingEnabled = true;
      context.imageSmoothingQuality = "high";
      context.drawImage(image, 0, 0, canvas.width, canvas.height);
      return canvas.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
    },
    { encoded: source.toString("base64"), scale: 3 },
  );
  await writeFile(outputPath, Buffer.from(scaledBase64, "base64"));
}

async function waitForFreshStandInBeat(page, clockStartedAt) {
  await page.waitForFunction(
    async (readyAt) => {
      const { WALK_STAND_IN_BEAT_SECONDS } = await import("/src/walk/beat.ts");
      const beatMilliseconds = WALK_STAND_IN_BEAT_SECONDS * 1_000;
      const phase = (performance.now() - readyAt) % beatMilliseconds;
      return phase >= 0 && phase < beatMilliseconds * 0.08;
    },
    clockStartedAt,
    { timeout: 4_000 },
  );
}

async function measureFrames(page) {
  return page.evaluate(
    () =>
      new Promise((resolve) => {
        const deltas = [];
        const renderTimes = [];
        let previous;
        let calls = 0;
        const sample = (time) => {
          if (previous !== undefined) {
            deltas.push(time - previous);
            const stage = document.querySelector("#feel-stage");
            renderTimes.push(Number(stage?.dataset.renderMilliseconds ?? "NaN"));
            calls = Number(stage?.dataset.renderCalls ?? "NaN");
          }
          previous = time;
          if (deltas.length === 30) {
            resolve({
              calls,
              averageRafMilliseconds:
                deltas.reduce((total, delta) => total + delta, 0) / deltas.length,
              averageRenderMilliseconds:
                renderTimes.reduce((total, duration) => total + duration, 0) /
                renderTimes.length,
            });
            return;
          }
          requestAnimationFrame(sample);
        };
        requestAnimationFrame(sample);
      }),
  );
}

async function walkRoute(page, clockStartedAt, from, to, pace, captureName = null) {
  const target = await groundCellPoint(page, from, to);
  await page.mouse.move(target.x, target.y);
  assert.equal(await stageAttribute(page, "data-walk-cursor"), "ready");
  await assertCustomCursor(page, "default");
  await page.mouse.click(target.x, target.y);
  assert.equal(await stageAttribute(page, "data-walk-state"), "draft");
  assert.equal(await stageAttribute(page, "data-walk-pace"), pace);
  assert.match(
    await page.locator(".walk-experiment-label").textContent(),
    new RegExp(` · ${pace.toUpperCase()}$`),
  );
  if (captureName !== null) {
    await page.evaluate(
      () =>
        new Promise((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(resolve)),
        ),
    );
    if (captureName === "walk-preview.png") {
      // Chromium's first WebGL readback can precede a textured card upload.
      // Warm the readback so both review captures contain the route's origin.
      await page.screenshot();
      await captureThreeTimesRouteCrop(
        page,
        from,
        to,
        path.join(captureRoot, "walk-footprints-crop.png"),
      );
    }
    await page.screenshot({ path: path.join(captureRoot, captureName) });
  }

  await waitForFreshStandInBeat(page, clockStartedAt);
  await page.mouse.dblclick(target.x, target.y);
  await assertCommittedPresentation(page, pace);
  await page.waitForFunction(
    (cell) => document.querySelector("#feel-stage")?.dataset.caretakerCell === cell,
    `${to.i},${to.j}`,
    { timeout: 4_000 },
  );
  await page.waitForFunction(
    () => document.querySelector("#feel-stage")?.dataset.walkState === "idle",
  );
  assert.equal(await page.locator("#feel-stage").getAttribute("data-walk-pace"), null);
  assert.equal(
    await page.locator(".walk-experiment-label").textContent(),
    "WALK EXPERIMENT — LOCAL, NOT AUTHORITY",
  );
  assert.deepEqual(parseCell(await stageAttribute(page, "data-caretaker-cell")), to);
  assert.equal(await stageAttribute(page, "data-walk-cursor"), "ready");
  await assertCustomCursor(page, "default");
  await assertCaretakerCentred(page);
}

const scratchRoot = await mkdtemp(path.join(tmpdir(), "tme-walk-camera-"));
const scratchPacket = path.join(scratchRoot, "packet");
await cp(assetRoot, scratchPacket, { recursive: true });
await copyFile(grownManifest, path.join(scratchPacket, "feel-manifest.json"));

const port = await freePort();
const baseUrl = `http://127.0.0.1:${port}/`;
let serverOutput = "";
const server = spawn(
  process.execPath,
  [
    path.join(webRoot, "node_modules/vite/bin/vite.js"),
    "--host",
    "127.0.0.1",
    "--port",
    String(port),
    "--strictPort",
  ],
  {
    cwd: webRoot,
    env: { ...process.env, TME_FEEL_ASSETS: scratchPacket },
    stdio: ["ignore", "pipe", "pipe"],
  },
);
server.stdout.on("data", (chunk) => {
  serverOutput += chunk.toString();
});
server.stderr.on("data", (chunk) => {
  serverOutput += chunk.toString();
});

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
  const clockStartedAt = Number(await stageAttribute(page, "data-walk-clock-started-at"));
  assert.ok(Number.isFinite(clockStartedAt));
  await page.waitForFunction(
    () => document.querySelector("#feel-stage")?.dataset.walkState === "idle",
  );
  const initial = parseCell(await stageAttribute(page, "data-caretaker-cell"));
  await assertCaretakerCentred(page);
  assert.equal(await page.locator(".walk-beat-meter").count(), 0);

  const authorable = { i: initial.i + 3, j: initial.j };
  const authorablePoint = await groundCellPoint(page, initial, authorable);
  await page.mouse.move(authorablePoint.x, authorablePoint.y);
  assert.equal(await stageAttribute(page, "data-walk-cursor"), "ready");
  await assertCustomCursor(page, "default");

  const refused = await visibleGroundCellPoint(page, initial, [
    { i: initial.i + 5, j: initial.j },
    { i: initial.i, j: initial.j + 5 },
    { i: initial.i + 5, j: initial.j + 5 },
    { i: initial.i - 5, j: initial.j + 5 },
    { i: initial.i - 5, j: initial.j },
    { i: initial.i, j: initial.j - 5 },
  ]);
  await page.mouse.move(refused.x, refused.y);
  assert.equal(await stageAttribute(page, "data-walk-cursor"), "refused");
  await assertCustomCursor(page, "not-allowed");

  const measurement = await measureFrames(page);
  assert.ok(Number.isFinite(measurement.calls) && measurement.calls > 0);
  assert.ok(Number.isFinite(measurement.averageRafMilliseconds));
  assert.ok(Number.isFinite(measurement.averageRenderMilliseconds));

  const sprintEnd = authorable;
  await walkRoute(page, clockStartedAt, initial, sprintEnd, "sprint", "walk-preview.png");
  await page.screenshot({ path: path.join(captureRoot, "walk-landed.png") });

  const walkEnd = { i: sprintEnd.i, j: sprintEnd.j + 1 };
  await walkRoute(page, clockStartedAt, sprintEnd, walkEnd, "walk");

  const runEnd = { i: walkEnd.i - 2, j: walkEnd.j };
  await walkRoute(page, clockStartedAt, walkEnd, runEnd, "run");
  await page.screenshot({ path: path.join(captureRoot, "walk-world.png") });

  assert.deepEqual(consoleErrors, []);
  console.log(
    `PASS walk proof: ${captureRoot}; draw calls ${measurement.calls}; ` +
      `30-frame rAF average ${measurement.averageRafMilliseconds.toFixed(3)} ms; ` +
      `render average ${measurement.averageRenderMilliseconds.toFixed(3)} ms`,
  );
} finally {
  await browser?.close();
  await stopServer(server);
  await rm(scratchRoot, { recursive: true, force: true });
}
