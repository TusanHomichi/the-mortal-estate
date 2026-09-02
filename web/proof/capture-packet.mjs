#!/usr/bin/env node
// Photograph the feel scene as a real tab sees it, for owner comparison:
// the packet from TME_FEEL_ASSETS, one screenshot per --query, zero console
// errors required. Usage:
//   TME_FEEL_ASSETS=<packet> node web/proof/capture-packet.mjs --out <dir> \
//       [--query preset=night] [--query "preset=night&zoom=-1"] [--width 1280] [--height 800]
import { mkdir } from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";
import { requirePacket, startVite } from "./serve.mjs";

function parseArguments(argv) {
  const options = { out: "", queries: [], width: 1280, height: 800 };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (flag === "--out") { options.out = value ?? ""; index += 1; }
    else if (flag === "--query") { options.queries.push(value ?? ""); index += 1; }
    else if (flag === "--width") { options.width = Number(value); index += 1; }
    else if (flag === "--height") { options.height = Number(value); index += 1; }
    else throw new Error(`unknown argument ${flag}`);
  }
  if (options.out === "") throw new Error("--out <directory> is required");
  if (options.queries.length === 0) options.queries.push("preset=night");
  if (!Number.isInteger(options.width) || !Number.isInteger(options.height)) {
    throw new Error("--width and --height must be whole pixel counts");
  }
  return options;
}

const options = parseArguments(process.argv.slice(2));
const packet = requirePacket();
await mkdir(options.out, { recursive: true });
const vite = await startVite(packet);
let failures = 0;
let browser;
try {
  browser = await chromium.launch({ executablePath: chromium.executablePath(), headless: true });
  for (const query of options.queries) {
    const name = query.replace(/[^A-Za-z0-9]+/g, "-").replace(/^-|-$/g, "") || "default";
    const page = await browser.newPage({
      viewport: { width: options.width, height: options.height },
      deviceScaleFactor: 1,
    });
    const errors = [];
    page.on("pageerror", (error) => errors.push(`page: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(`console: ${message.text()}`);
    });
    try {
      await page.goto(`${vite.baseUrl}?${query}`, { waitUntil: "networkidle", timeout: 30_000 });
      await page.waitForFunction(() => document.body.dataset.sceneReady === "true", null, { timeout: 30_000 });
      const state = await page.locator("#feel-stage").getAttribute("data-scene-state");
      if (state !== "ready") errors.push(`scene: ${await page.locator("#scene-banner").innerText()}`);
      await page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      const target = path.join(options.out, `${name}.png`);
      await page.screenshot({ path: target });
      console.log(`${errors.length === 0 && state === "ready" ? "CAPTURED" : "FAILED"} ${target} ?${query}${errors.length ? "\n  " + errors.join("\n  ") : ""}`);
      if (errors.length > 0 || state !== "ready") failures += 1;
    } catch (error) {
      failures += 1;
      console.log(`FAILED ?${query}: ${error instanceof Error ? error.message : String(error)}`);
    } finally {
      await page.close();
    }
  }
} finally {
  await browser?.close();
  await vite.stop();
}
process.exitCode = failures === 0 ? 0 : 1;
