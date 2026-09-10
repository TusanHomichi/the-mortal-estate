#!/usr/bin/env node
// Native form safety while JavaScript is absent or protocol startup is stalled.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { launchProofBrowser, proofBrowsers } from "./serve.mjs";
let input = ""; for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input), reports = [];
for (const spec of proofBrowsers()) {
  const launched = await launchProofBrowser({ ...spec, trustedAuthority: config.authority });
  try {
    // The trusted Firefox launcher returns its persistent context. New contexts
    // still belong to that browser and inherit its installed certificate trust.
    const browser = launched.context ? launched.context.browser() : launched.browser;
    if (!browser) throw new Error("Browser contexts are unavailable");
    const offline = await browser.newContext({ javaScriptEnabled: false });
    const page = await offline.newPage(); let crashed = false, offlineVerdict = "PASS";
    page.on("crash", () => { crashed = true; });
    try {
      await page.goto(config.origin + "/index.html");
      assert(await page.locator("#login").isDisabled());
      assert(await page.locator("#username").isDisabled());
      assert(await page.locator("#password").isDisabled());
      assert.equal(await page.locator("#login-form").getAttribute("method"), "post");
      let submissions = 0;
      page.on("request", request => { if (request.isNavigationRequest()) ++submissions; });
      const attempt = await page.locator("#login-form").evaluate(form => {
        const data = [...new FormData(form).entries()];
        let refusal = null;
        try { form.submit(); }
        catch (error) { refusal = { name: error.name, message: error.message }; }
        return { data, refusal };
      });
      await page.waitForTimeout(150);
      assert.deepEqual(attempt.data, []); assert.equal(submissions, 0);
    } catch (error) {
      // This native WebKit crash is independently reproduced by one fixed-position
      // section and a button with JS disabled. Never count it as a safety pass.
      if (spec.name !== "webkit" || !crashed) throw error;
      offlineVerdict = "UNAVAILABLE";
      console.log("UNAVAILABLE WebKit JavaScript-disabled page: native page crash (see game-entry execution record)");
    } finally { await offline.close(); }

    const live = await browser.newContext();
    const delayed = await live.newPage(); let failCodec, codecArrived;
    const intercepted = new Promise(resolve => { codecArrived = resolve; });
    await delayed.route("**/codec.wasm", route => new Promise(resolve => {
      failCodec = async () => { await route.fulfill({ status: 503, body: "Synthetic startup failure" }); resolve(); };
      codecArrived();
    }));
    await delayed.goto(config.origin + "/index.html"); await intercepted;
    assert(await delayed.locator("#login").isDisabled());
    assert(await delayed.locator("#password").isDisabled());
    await failCodec();
    await delayed.waitForFunction(() => document.body.dataset.playReady === "failed", undefined, { polling: 50, timeout: 30_000 });
    assert(await delayed.locator("#login").isDisabled());
    assert.match(await delayed.locator("#connection").textContent(), /could not load/);
    assert.equal(new URL(delayed.url()).search, "");
    await live.close();
    reports.push({ engine: spec.name, javascript_absent: offlineVerdict, native_submission_blocked: offlineVerdict, delayed_codec: "PASS", failed_startup: "PASS" });
    console.log(`PASS enabled startup safety: ${spec.name}; JavaScript absent: ${offlineVerdict}`);
  } finally { await launched.stop(); }
}
const complete = reports.every(report => report.javascript_absent === "PASS");
await writeFile(config.output, JSON.stringify({ verdict: complete ? "PASS" : "INCOMPLETE", enabled_startup: "PASS", reports }, null, 2) + "\n");
if (!complete) process.exitCode = 3;
