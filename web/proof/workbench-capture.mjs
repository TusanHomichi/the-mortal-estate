#!/usr/bin/env node
// Exercise the restored button and file-only selection through the served UI.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { launchProofBrowser, proofBrowsers } from "./serve.mjs";

const { base, output } = JSON.parse(await new Promise(resolve => {
  let input = ""; process.stdin.on("data", chunk => { input += chunk; }); process.stdin.on("end", () => resolve(input));
}));
let offered;
const engines = proofBrowsers();
if (engines.length !== 2) throw new Error("Workbench capture proof requires both browser engines");
for (const engine of engines) {
  const launched = await launchProofBrowser(engine);
  try {
    const page = await launched.browser.newPage({ viewport: { width: 1280, height: 800 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(base);
    await page.waitForFunction(() => !document.getElementById("take-capture").disabled);
    if (!offered) {
      const response = page.waitForResponse(row => row.url() === `${base}/api/capture` && row.request().method() === "POST", { timeout: 600_000 });
      await page.locator("#take-capture").click();
      const result = await response;
      assert.equal(result.status(), 201);
      offered = (await result.json()).captures;
      assert.equal(offered.length, 4);
      await page.waitForFunction(() => !document.getElementById("take-capture").disabled, null, { timeout: 30_000 });
      assert.match(await page.locator("#capture-status").textContent(), /^4 captures in \d+\.\d s$/);
    } else {
      await page.locator('button[data-view="capture"]').click();
    }
    const stateHandle = await page.evaluateHandle(async () => (await import("/static/state.js")).state);
    await page.waitForFunction(state => state.view === "capture" && Boolean(state.captureImage), stateHandle);
    const point = await page.evaluate(state => {
      const canvas = document.getElementById("canvas"), box = canvas.getBoundingClientRect();
      return { x: box.x + (state.origin.x + state.captureImage.width / 2 * state.scale) * box.width / canvas.width,
        y: box.y + (state.origin.y + state.captureImage.height / 2 * state.scale) * box.height / canvas.height };
    }, stateHandle);
    await stateHandle.dispose();
    const preview = page.waitForResponse(row => row.url() === `${base}/api/capture/preview` && row.request().method() === "POST");
    await page.mouse.click(point.x, point.y);
    assert.equal((await preview).status(), 200);
    const recorded = page.waitForResponse(row => row.url() === `${base}/api/capture/selection` && row.request().method() === "POST");
    await page.locator("#record").click();
    const recordResult = await recorded;
    assert.equal(recordResult.status(), 201, JSON.stringify(await recordResult.json()));
    assert.deepEqual(errors, []);
    console.log(`PASS ${engine.name}: Workbench capture view, real canvas selection and record button`);
    await page.close();
  } finally { await launched.stop(); }
}
await writeFile(output, JSON.stringify({ captures: offered, engines: ["chromium", "firefox"], button_and_selection: true }) + "\n");
