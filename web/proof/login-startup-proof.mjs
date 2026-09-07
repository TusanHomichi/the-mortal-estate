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
    const offline = await launched.browser.newContext({ javaScriptEnabled: false });
    const page = await offline.newPage();
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
      try { form.submit(); } // bypass the missing handler: document policy must still block it.
      catch (error) { refusal = { name: error.name, message: error.message }; }
      return { data, refusal };
    });
    await page.waitForTimeout(150);
    assert.deepEqual(attempt.data, []); assert.equal(submissions, 0);

    await offline.close();

    const live = await launched.browser.newContext();
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
    reports.push({ engine: spec.name, javascript_absent: "PASS", native_submission_blocked: "PASS", delayed_codec: "PASS", failed_startup: "PASS" });
    console.log(`PASS login startup safety: ${spec.name}`);
  } finally { await launched.stop(); }
}
await writeFile(config.output, JSON.stringify({ verdict: "PASS", reports }, null, 2) + "\n");
