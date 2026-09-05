#!/usr/bin/env node
// Local-only driver. The live browser uses native WSS; Playwright supplies only
// the diagnostic page at the scratch server's origin, never the gameplay wire.
import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { build } from "vite";
import { launchProofBrowser, PROOF_ENGINES, webRoot } from "./serve.mjs";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input);
input = "";
if (!PROOF_ENGINES[config.engine]) throw new Error("unknown browser engine");
const built = await build({ configFile: false, root: webRoot, logLevel: "error",
  build: { write: false, target: "es2022", minify: false,
    lib: { entry: path.join(webRoot, "src/authoritative/main.ts"), formats: ["es"] } } });
const output = (Array.isArray(built) ? built[0] : built).output;
const script = output.find(row => row.type === "chunk" && row.isEntry).code;
const wasm = await readFile(path.join(process.env.CARGO_TARGET_DIR || path.join(webRoot, "../target"), "wasm32-unknown-unknown/release/tme_protocol.wasm"));
const engine = PROOF_ENGINES[config.engine];
const launched = await launchProofBrowser({ name: config.engine, engine, executablePath: engine.executablePath() });
try {
  // Certificate errors are permitted only in this ephemeral scratch profile;
  // this proof does not claim production browser certificate verification.
  const context = await launched.browser.newContext({ viewport: { width: 768, height: 512 }, deviceScaleFactor: 1, ignoreHTTPSErrors: true });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  page.on("console", message => { if (message.type() === "error") console.error(message.text()); });
  const origin = config.origin || "https://localhost:443";
  if (config.engine === "chromium") await context.grantPermissions(["local-network-access"], { origin });
  await page.route(`${origin}/capture`, route => route.fulfill({ contentType: "text/html", body: '<!doctype html><html><head><meta charset="utf-8"><title>Authoritative capture</title></head><body><script type="module" src="/capture.js"></script></body></html>' }));
  await page.route(`${origin}/capture.js`, route => route.fulfill({ contentType: "application/javascript", body: script }));
  await page.route(`${origin}/codec.wasm`, route => route.fulfill({ contentType: "application/wasm", body: wasm }));
  if (config.ticket) await page.goto(`${origin}/health/live`);
  await page.goto(`${origin}/capture`);
  await page.waitForFunction(() => document.body.dataset.captureReady === "true");
  if (config.ticket) {
    await page.evaluate(({ origin, ticket }) => window.authoritativeCapture.connect(origin.replace("https:", "wss:") + "/v4/socket", ticket), { origin, ticket: config.ticket });
    delete config.ticket;
    await page.waitForFunction(() => window.authoritativeCapture.snapshot?.generation >= 2, null, { timeout: 30_000 });
  } else {
    await page.evaluate(messages => window.authoritativeCapture.replay(messages), config.envelopes);
  }

  async function capture(route, directory) {
    const captured = await page.evaluate(({ route, sources }) => window.authoritativeCapture.capture(route, sources), { route, sources: config.sources });
    const samples = Buffer.from(captured.raster).subarray(Buffer.from(captured.raster).indexOf("65535\n") + 6);
    const probes = await page.evaluate(() => {
      const api = window.authoritativeCapture, points = [];
      for (let y = 0; y < innerHeight; y += 7) for (let x = 0; x < innerWidth; x += 7) points.push([x, y]);
      for (const row of api.targets) {
        const r = row.hit_shape;
        points.push([row.anchor.x, row.anchor.y]);
        for (const x of [r.x - 1, r.x, r.x + r.width - 1, r.x + r.width]) {
          for (const y of [r.y - 1, r.y, r.y + r.height - 1, r.y + r.height]) points.push([x, y]);
        }
      }
      return points.filter(([x, y]) => x >= 0 && y >= 0 && x < innerWidth && y < innerHeight)
        .map(([x, y]) => ({ x, y, index: api.pointer(x + .5, y + .5)?.index ?? 0 }));
    });
    for (const probe of probes) assert.equal(samples.readUInt16BE((probe.y * 768 + probe.x) * 2), probe.index, `pointer/raster disagreement at ${probe.x},${probe.y}`);
    for (const target of captured.sidecar.targets) {
      const { x, y } = target.anchor;
      assert.equal(samples.readUInt16BE((y * 768 + x) * 2), target.index, "target anchor obscured");
      await page.mouse.move(x + .5, y + .5);
      assert.equal(await page.locator("canvas").getAttribute("data-pointer-identity"), target.identity, "actual pointer handler disagrees");
    }
    await mkdir(directory, { recursive: true });
    await writeFile(path.join(directory, "capture.png"), Buffer.from(captured.image));
    await writeFile(path.join(directory, "capture.identity.pgm"), Buffer.from(captured.raster));
    await writeFile(path.join(directory, "capture.frame.json"), Buffer.from(captured.recording));
    await writeFile(path.join(directory, "capture.sidecar.json"), JSON.stringify(captured.sidecar, null, 2) + "\n");
    return { captured, probes: probes.length };
  }

  const firstRoute = config.envelopes ? "replay" : "live";
  const first = await capture(firstRoute, path.join(config.output, firstRoute));
  const recording = JSON.parse(Buffer.from(first.captured.recording));
  await page.evaluate(messages => window.authoritativeCapture.replay(messages), recording.envelopes);
  const replay = await capture("replay", path.join(config.output, "replay"));
  for (const field of ["image", "raster", "recording"]) assert.deepEqual(replay.captured[field], first.captured[field], `replay ${field} differs`);
  for (const field of ["targets", "camera", "scene", "frame_generation", "authority"]) assert.deepEqual(replay.captured.sidecar[field], first.captured.sidecar[field], `replay ${field} differs`);
  await page.evaluate(() => window.authoritativeCapture.disconnect());
  assert.equal(await page.evaluate(() => window.authoritativeCapture.snapshot), null);
  assert.deepEqual(errors, []);
  await writeFile(path.join(config.output, "proof.json"), JSON.stringify({ engine: config.engine, renderer: launched.renderer,
    route: firstRoute, pointer_probes: first.probes + replay.probes, targets: first.captured.sidecar.targets.length,
    exact_replay: true, disconnect_clears_authority: true }, null, 2) + "\n");
  console.log(`PASS ${config.engine}: ${firstRoute}/replay image, identity, frame, pointer correspondence`);
  await context.close();
} finally { await launched.stop(); }
