import assert from "node:assert/strict";
import { mkdir, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { execFile, spawn } from "node:child_process";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";
import { ProofUnavailable, launchProofBrowser, proofBrowsers, reportUnavailable, requirePacket, startVite } from "./serve.mjs";

// The packet under proof comes from the environment, never a tracked path; the
// captures go where the owner asks (the capture-output capability) or to a
// temporary directory that the summary line names.
const packet = requirePacket();
const engines = proofBrowsers();
const captureBase = path.resolve(
  process.env.TME_CAPTURE_OUTPUT?.trim() || path.join(os.tmpdir(), "tme-walk-proof"),
);

// Two engines, two processes: the proof below is written once, for one
// browser, and the parent runs it per engine so each gets a fresh server, a
// fresh tab, and its own capture directory.
if (engines.length > 1) {
  const script = fileURLToPath(import.meta.url);
  let failed = 0;
  let unavailable = 0;
  for (const { name } of engines) {
    const exit = await new Promise((resolve) => {
      const child = spawn(process.execPath, [script], {
        stdio: "inherit",
        env: { ...process.env, TME_PROOF_BROWSER: name, TME_CAPTURE_OUTPUT: path.join(captureBase, name) },
      });
      child.on("exit", (code) => resolve(code ?? 1));
    });
    if (exit === 3) unavailable += 1;
    else if (exit !== 0) failed += 1;
  }
  // A failed engine is a failure; an engine that could not run is an
  // incomplete proof, and stays exit 3 all the way up.
  if (failed > 0) {
    console.log(`FAIL walk proof: ${failed} of ${engines.length} engines failed`);
    process.exit(1);
  }
  if (unavailable > 0) {
    console.error(`UNAVAILABLE: walk proof ran in ${engines.length - unavailable} of ${engines.length} engines`);
    process.exit(3);
  }
  console.log(`PASS walk proof in ${engines.map((e) => e.name).join(" and ")}: ${captureBase}`);
  process.exit(0);
}
const [proofEngine] = engines;
const engineName = proofEngine.name;
const captureRoot = captureBase;
const sequenceFrames = path.join(captureRoot, ".wind-sequence-frames");
const execFileAsync = promisify(execFile);
// Two beats and slack for a landing, plus the synchronous build of the next
// space when the landing is a portal: measured at up to ~9 s on headless
// Chromium's software GL, under 2 s on Firefox's.
const SPACE_BUILD_DEADLINE_MS = 20_000;

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

// The scene names its camera focus (the caretaker outdoors, the room's centre
// inside a building); the click for a target cell is computed from that, not
// from a guess about whom the camera follows.
async function groundCellPoint(page, _focus, target) {
  return page.evaluate(
    async ({ targetCell }) => {
      const { createFeelCamera } = await import("/src/camera.ts");
      const [fi, fj] = document.querySelector("#feel-stage").dataset.cameraFocus.split(",").map(Number);
      const camera = createFeelCamera(innerWidth, innerHeight, { i: fi, j: fj });
      const projected = camera.position.clone().set(targetCell.i, 0, targetCell.j).project(camera);
      return {
        x: (projected.x + 1) * innerWidth * 0.5,
        y: (1 - projected.y) * innerHeight * 0.5,
      };
    },
    { targetCell: target },
  );
}

// The fresh-beat window is the first 8 % of a beat — 240 ms. Polled on
// animation frames it can be missed on every beat by a rasteriser whose
// frames are 300 ms apart (headless Chromium here), so it polls on a timer.
async function waitForFreshStandInBeat(page, clockStartedAt) {
  await page.waitForFunction(
    async (readyAt) => {
      const { WALK_STAND_IN_BEAT_SECONDS } = await import("/src/walk/beat.ts");
      const nowSeconds = performance.now() / 1_000;
      const phase = (nowSeconds - readyAt) % WALK_STAND_IN_BEAT_SECONDS;
      return phase >= 0 && phase < WALK_STAND_IN_BEAT_SECONDS * 0.08;
    },
    clockStartedAt,
    { polling: 16, timeout: 8_000 }, // two beats and slack
  );
}

async function assertCaretakerCentred(page) {
  const projection = await stageAttribute(page, "data-caretaker-projection");
  const [x, y] = projection.split(",").map(Number);
  assert.ok(Math.abs(x - 640) <= 1, `caretaker projects to x=${x}, not viewport centre`);
  assert.ok(Math.abs(y - 400) <= 1, `caretaker projects to y=${y}, not viewport centre`);
}

async function measureFrames(page) {
  return page.evaluate(
    () => new Promise((resolve) => {
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

async function captureWindSequence(page) {
  await rm(sequenceFrames, { recursive: true, force: true });
  await mkdir(sequenceFrames, { recursive: true });
  const startedAt = await page.evaluate(() => performance.now());
  for (let index = 0; index < 12; index += 1) {
    await page.waitForFunction(
      ({ start, target }) => performance.now() - start >= target,
      { start: startedAt, target: index * 250 },
    );
    await page.screenshot({
      path: path.join(sequenceFrames, `wind-frame-${String(index).padStart(2, "0")}.png`),
    });
  }
  const output = path.join(captureRoot, "walk-wind-sequence.webp");
  await execFileAsync("ffmpeg", [
    "-loglevel", "error",
    "-y",
    "-framerate", "4",
    "-i", path.join(sequenceFrames, "wind-frame-%02d.png"),
    "-frames:v", "12",
    "-an",
    "-c:v", "libwebp_anim",
    "-quality", "82",
    "-loop", "0",
    output,
  ]);
  const { stdout } = await execFileAsync("magick", [
    "identify",
    "-format", "%n %T\n",
    output,
  ]);
  const frames = stdout.trim().split("\n");
  assert.equal(frames.length, 12);
  assert.ok(frames.every((frame) => frame === "12 25"));
  await rm(sequenceFrames, { recursive: true, force: true });
}

async function draftAndCommit(page, clockStartedAt, from, target) {
  const point = await groundCellPoint(page, from, target);
  const spaceBefore = await stageAttribute(page, "data-walk-space");
  await page.mouse.move(point.x, point.y);
  assert.equal(await stageAttribute(page, "data-walk-cursor"), "ready");
  await page.mouse.click(point.x, point.y);
  assert.equal(await stageAttribute(page, "data-walk-state"), "draft");
  const trace = [];
  const snap = async (label) => trace.push(`${label}: ${await page.locator("#feel-stage").evaluate((stage) => `${stage.dataset.walkState}@${stage.dataset.caretakerCell} cursor=${stage.dataset.walkCursor} t=${(performance.now() / 1000).toFixed(3)}`)}`);
  await snap("after click");
  await waitForFreshStandInBeat(page, clockStartedAt);
  await snap("after fresh beat");
  await page.locator("#feel-stage").evaluate((stage, position) => {
    const canvas = stage.querySelector("canvas");
    if (!(canvas instanceof HTMLCanvasElement)) throw new Error("the proof found no canvas");
    canvas.dispatchEvent(new MouseEvent("dblclick", {
      bubbles: true,
      cancelable: true,
      button: 0,
      buttons: 1,
      clientX: position.x,
      clientY: position.y,
      detail: 2,
    }));
  }, point);
  // The commitment is observed, not sampled, and on a timer: under a slow
  // rasteriser the beat can strike between the double-click and the next
  // animation frame, by which time the route has landed — on the target
  // cell, or through a portal into another space. Each of those is the
  // commitment succeeding; a strict re-read of "committed" was #31.
  try {
    await page.waitForFunction(
      ({ expectedCell, previousSpace }) => {
        const stage = document.querySelector("#feel-stage");
        if (stage === null) return false;
        const { walkState, walkSpace, caretakerCell } = stage.dataset;
        return walkState === "committed" ||
          (walkState === "idle" && caretakerCell === expectedCell) ||
          (walkSpace !== previousSpace);
      },
      { expectedCell: `${target.i},${target.j}`, previousSpace: spaceBefore },
      // A landing through a portal builds the next space synchronously, and
      // under software GL that blocks the page for seconds — the poll cannot
      // run until it finishes, so the deadline must outlast a space build.
      { polling: 16, timeout: SPACE_BUILD_DEADLINE_MS },
    );
  } catch (error) {
    await snap("after double-click wait");
    throw new Error(`no commitment observed after the double-click; ${trace.join(" | ")}`, { cause: error });
  }
}

async function walkWithinSpace(page, clockStartedAt, space, from, target, cameraFollows = true) {
  await draftAndCommit(page, clockStartedAt, from, target);
  await page.waitForFunction(
    ({ expectedSpace, expectedCell }) => {
      const stage = document.querySelector("#feel-stage");
      return stage?.dataset.walkSpace === expectedSpace &&
        stage.dataset.caretakerCell === expectedCell &&
        stage.dataset.walkState === "idle";
    },
    { expectedSpace: space, expectedCell: `${target.i},${target.j}` },
    { timeout: 8_000 }, // two beats and slack; the state and cell are still asserted
  );
  if (cameraFollows) await assertCaretakerCentred(page);
}

async function walkThroughPortal(
  page,
  clockStartedAt,
  from,
  door,
  targetSpace,
  targetCell,
) {
  await draftAndCommit(page, clockStartedAt, from, door);
  await page.waitForFunction(
    ({ expectedSpace, expectedCell }) => {
      const stage = document.querySelector("#feel-stage");
      return stage?.dataset.walkSpace === expectedSpace &&
        stage.dataset.caretakerCell === expectedCell &&
        stage.dataset.walkState === "idle";
    },
    { expectedSpace: targetSpace, expectedCell: `${targetCell.i},${targetCell.j}` },
    { polling: 16, timeout: SPACE_BUILD_DEADLINE_MS }, // the landing builds a space; state and cell are still asserted
  );
  assert.equal(await stageAttribute(page, "data-walk-space"), targetSpace);
  assert.deepEqual(parseCell(await stageAttribute(page, "data-caretaker-cell")), targetCell);
}

// Inside a building the camera belongs to the space (owner ruling, 2026-09-02):
// the focus is the room's centre, and a landing does not move it.
async function assertCameraBelongsToSpace(page, focus) {
  assert.equal(await stageAttribute(page, "data-camera-focus"), `${focus.i},${focus.j}`);
  const projection = await stageAttribute(page, "data-caretaker-projection");
  const [x, y] = projection.split(",").map(Number);
  assert.ok(Math.abs(x - 640) > 8 || Math.abs(y - 400) > 8, "the caretaker is centred, so the camera followed it indoors");
}

const vite = await startVite(packet);
const baseUrl = vite.baseUrl;

let launched;
try {
  await mkdir(captureRoot, { recursive: true });
  launched = await launchProofBrowser(proofEngine);
  const browser = launched.browser;
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  const consoleErrors = [];
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(`console: ${message.text()}`);
  });
  page.on("pageerror", (error) => consoleErrors.push(`page: ${error.message}`));

  await page.goto(`${baseUrl}?preset=wind`, { waitUntil: "networkidle" });
  await page.waitForFunction(() => document.body.dataset.sceneReady === "true");
  await page.waitForFunction(
    () => document.querySelector("#feel-stage")?.dataset.walkState === "idle",
  );
  const clockStartedAt = Number(await stageAttribute(page, "data-walk-clock-started-at"));
  assert.ok(Number.isFinite(clockStartedAt));
  assert.equal(await stageAttribute(page, "data-walk-space"), "estate-grounds");
  const grassInstances = Number(await stageAttribute(page, "data-grass-instances"));
  assert.ok(grassInstances > 0 && grassInstances <= 1_800);
  const start = parseCell(await stageAttribute(page, "data-caretaker-cell"));
  assert.deepEqual(start, { i: 13, j: 11 });
  await assertCaretakerCentred(page);

  await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  await page.screenshot({ path: path.join(captureRoot, "walk-wind.png") });
  await captureWindSequence(page);
  await page.screenshot({ path: path.join(captureRoot, "walk-roofs.png") });
  const measurement = await measureFrames(page);
  assert.ok(Number.isFinite(measurement.calls) && measurement.calls > 0);
  assert.ok(Number.isFinite(measurement.averageRafMilliseconds));
  assert.ok(Number.isFinite(measurement.averageRenderMilliseconds));

  const courtyardStaging = { i: 12, j: 8 };
  const doorApproach = { i: 11, j: 7 };
  await walkWithinSpace(page, clockStartedAt, "estate-grounds", start, courtyardStaging);
  await walkWithinSpace(page, clockStartedAt, "estate-grounds", courtyardStaging, doorApproach);
  await walkThroughPortal(
    page,
    clockStartedAt,
    doorApproach,
    { i: 11, j: 6 },
    "estate-ground-room",
    { i: 4, j: 3 },
  );
  await page.screenshot({ path: path.join(captureRoot, "walk-interior.png") });
  assert.equal(await stageAttribute(page, "data-grass-instances"), "0");
  const roomCentre = { i: 4, j: 2 };
  await assertCameraBelongsToSpace(page, roomCentre);
  await walkWithinSpace(page, clockStartedAt, "estate-ground-room", { i: 4, j: 3 }, { i: 3, j: 3 }, false);
  await assertCameraBelongsToSpace(page, roomCentre);
  await walkWithinSpace(page, clockStartedAt, "estate-ground-room", { i: 3, j: 3 }, { i: 4, j: 3 }, false);

  await walkThroughPortal(
    page,
    clockStartedAt,
    { i: 4, j: 3 },
    { i: 4, j: 4 },
    "estate-grounds",
    { i: 11, j: 7 },
  );
  await assertCaretakerCentred(page);
  await page.screenshot({ path: path.join(captureRoot, "walk-exterior-return.png") });
  assert.equal(Number(await stageAttribute(page, "data-grass-instances")), grassInstances);

  assert.deepEqual(consoleErrors, []);
  console.log(
    `PASS walk proof (${engineName}): ${captureRoot}; ${grassInstances} grass clumps; ` +
      `exterior draw calls ${measurement.calls}; ` +
      `30-frame rAF average ${measurement.averageRafMilliseconds.toFixed(3)} ms; ` +
      `render average ${measurement.averageRenderMilliseconds.toFixed(3)} ms`,
  );
} catch (error) {
  if (!(error instanceof ProofUnavailable)) throw error;
  reportUnavailable(error);
} finally {
  await launched?.stop();
  await vite.stop();
  await rm(sequenceFrames, { recursive: true, force: true });
}
