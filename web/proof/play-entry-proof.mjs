#!/usr/bin/env node
// Check renderer selection on the actual installed browser artifact, before login.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { launchProofBrowser, proofBrowsers } from "./serve.mjs";
let input = "";
for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input);
assert(["inspection", "pixel-art"].includes(config.presentation));
const reports = [];
for (const spec of proofBrowsers()) {
  const launched = await launchProofBrowser({ ...spec, trustedAuthority: config.authority });
  try {
    const page = await launched.browser.newPage({ viewport: { width: 1280, height: 900 } });
    const errors = []; page.on("pageerror", error => errors.push(error.message));
    for (const entry of ["/", "/index.html", "/index.html?study=diagnostic", "/index.html?study=first-expedition"]) {
      await page.goto(config.origin + entry);
      await page.waitForFunction(() => document.body.dataset.playReady === "true", undefined, { polling: 50, timeout: 60_000 });
      const presentation = await page.locator("#world-canvas").getAttribute("data-presentation");
      assert.equal(presentation, config.presentation === "pixel-art" ? "pixel-art" : null,
        `${spec.name}: ${entry} selected the wrong renderer`);
      reports.push({ engine: spec.name, entry, presentation });
    }
    assert.deepEqual(errors, []);
    console.log(`PASS deployed renderer entry paths: ${spec.name}`);
  } finally { await launched.stop(); }
}
await writeFile(config.output, JSON.stringify({ verdict: "PASS", reports }, null, 2) + "\n");
