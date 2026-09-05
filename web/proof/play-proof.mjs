#!/usr/bin/env node
// The deployed bundle and real UI own every control request and gameplay send.
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { launchProofBrowser, PROOF_ENGINES } from "./serve.mjs";
let input = "";
for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input); input = "";
const reports = [];
// A headed Firefox background tab can suspend animation-frame polling.
const wait = (page, predicate, argument) => page.waitForFunction(predicate, argument, { polling: 50, timeout: 30_000 });
for (const name of ["chromium", "firefox"]) {
  const engine = PROOF_ENGINES[name];
  const launched = await launchProofBrowser({ name, engine, executablePath: engine.executablePath(), trustedAuthority: config.authority, multipleWindows: true });
  try {
    const context = launched.context || await launched.browser.newContext({ viewport: { width: 1280, height: 720 } });
    const pages = await Promise.all([context.newPage(), context.newPage()]);
    for (const page of pages) await page.setViewportSize({ width: 1280, height: 720 });
    const errors = [], tokens = new Map(), socketHeaders = [];
    context.on("request", request => {
      if (request.url().endsWith("/socket")) socketHeaders.push(request.headers());
      if (/\/v4\//.test(request.url())) {
        const headers = request.headers();
        assert.equal(headers.cookie, undefined, "browser sent a cookie");
        const auth = headers.authorization;
        if (auth) tokens.set(request.frame().page(), auth);
      }
    });
    for (let index = 0; index < 2; ++index) {
      const page = pages[index];
      page.on("pageerror", error => { errors.push(error.message); console.error(`${name} page error: ${error.message}`); });
      await page.bringToFront();
      await page.goto(config.origin);
      await wait(page, () => document.body.dataset.playReady === "true");
      await page.locator("#username").fill(config.accounts[index].username);
      await page.locator("#password").fill(config.accounts[index].password);
      await page.getByRole("button", { name: "Sign in", exact: true }).click();
      await wait(page, () => document.body.dataset.phase === "selecting");
      assert.equal(await page.locator("#password").inputValue(), "");
      await page.getByRole("button", { name: "Enter world" }).click();
      await wait(page, () => document.body.dataset.phase === "playing");
    }
    const facts = page => page.locator("#world-canvas").evaluate(canvas => ({ ...canvas.dataset }));
    const ready = page => wait(page, () => document.querySelector("#world-canvas").dataset.canAct === "true");
    const first = await facts(pages[0]), second = await facts(pages[1]);
    assert.notEqual(first.actor, second.actor);
    assert(tokens.get(pages[0]) && tokens.get(pages[1]) && tokens.get(pages[0]) !== tokens.get(pages[1]), "tabs shared authentication");
    assert.equal((await context.cookies()).length, 0);
    for (let i = 0; i < 2; i++) {
      const row = await pages[i].locator("#occupants li").filter({ hasText: "(you)" }).textContent();
      assert((await pages[1-i].locator("#occupants").textContent()).includes(row.replace(" (you)", "")), "other tab did not observe this character");
    }
    // Offset actual UI wait actions. No proof-side command injection.
    await ready(pages[0]); await ready(pages[1]);
    await pages[0].getByRole("button", { name: "Wait", exact: true }).click();
    await wait(pages[0], () => document.querySelector("#world-canvas").dataset.canAct === "false");
    const oneBusy = await facts(pages[0]);
    await new Promise(resolve => setTimeout(resolve, 900));
    await pages[1].getByRole("button", { name: "Wait", exact: true }).click();
    await wait(pages[1], () => document.querySelector("#world-canvas").dataset.canAct === "false");
    const twoBusy = await facts(pages[1]);
    const offset = BigInt(twoBusy.readyAt) - BigInt(oneBusy.readyAt);
    assert(offset > 300n && offset < 2200n, "UI actions did not receive independent offset deadlines");
    assert(await pages[0].getByRole("button", { name: "North", exact: true }).isDisabled());
    await pages[0].getByRole("button", { name: "Reconnect", exact: true }).click();
    await wait(pages[0], () => document.body.dataset.phase === "playing");
    assert.equal((await facts(pages[0])).readyAt, oneBusy.readyAt);
    await ready(pages[0]);
    assert.equal((await facts(pages[1])).canAct, "false");
    await ready(pages[1]);
    // Actual movement via both button and keyboard, with authoritative position
    // changes and the other tab's visible occupant list reconciling afterward.
    for (let index = 0; index < 2; ++index) {
      const page = pages[index], other = pages[1 - index];
      let moved = false;
      for (const direction of ["North", "South", "West", "East"]) {
        await ready(page);
        const before = await page.locator("#position").textContent();
        if (index === 0) await page.getByRole("button", { name: direction, exact: true }).click();
        else { await page.locator("#world-canvas").focus(); await page.keyboard.press({ North: "ArrowUp", South: "ArrowDown", West: "ArrowLeft", East: "ArrowRight" }[direction]); }
        await wait(page, () => document.querySelector("#world-canvas").dataset.pending === "false");
        await page.waitForTimeout(200);
        if (await page.locator("#position").textContent() !== before) { moved = true; break; }
      }
      assert(moved, "no UI movement changed authoritative position");
      const row = await page.locator("#occupants li").filter({ hasText: "(you)" }).textContent();
      await wait(other, expected => document.querySelector("#occupants").textContent.includes(expected), row.replace(" (you)", ""));
    }
    // Preferences persist alone, with all essential controls inside the viewport
    // horizontally at the minimum size and 200% text. Vertical scrolling is allowed.
    await pages[0].locator("#settings summary").click();
    await pages[0].locator("#text-scale").selectOption("200");
    const overflow = await pages[0].evaluate(() => document.documentElement.scrollWidth > innerWidth);
    assert.equal(overflow, false, "200% text clips horizontally");
    await mkdir(config.output, { recursive: true });
    await pages[0].screenshot({ path: path.join(config.output, `${name}-200-percent.png`), fullPage: true });
    const storage = await pages[0].evaluate(() => ({ local: { ...localStorage }, session: { ...sessionStorage } }));
    assert.deepEqual(Object.keys(storage.local), ["tme.play.preferences"]); assert.deepEqual(storage.session, {});
    await pages[0].locator("#text-scale").selectOption("100");
    await pages[0].locator("#settings summary").click();
    await pages[0].screenshot({ path: path.join(config.output, `${name}-play.png`), fullPage: true });
    const oldToken = tokens.get(pages[0]);
    await pages[0].getByRole("button", { name: "Sign out", exact: true }).click();
    await wait(pages[0], () => document.body.dataset.phase === "signed_out");
    const revoked = await pages[0].evaluate(async authorization => (await fetch("/v4/session", { method: "POST", credentials: "omit", headers: { Authorization: authorization, "Content-Type": "application/json" }, body: "{}" })).status, oldToken);
    assert.equal(revoked, 401);
    assert.equal((await facts(pages[0])).actor, "");
    assert.equal((await facts(pages[1])).actor, second.actor, "one tab's logout disturbed the other");
    await ready(pages[1]);
    await pages[1].getByRole("button", { name: "Wait", exact: true }).click();
    await wait(pages[1], () => document.querySelector("#world-canvas").dataset.pending === "false" && document.querySelector("#feedback").textContent === "Action accepted.");
    await pages[1].getByRole("button", { name: "Sign out", exact: true }).click();
    await wait(pages[1], () => document.body.dataset.phase === "signed_out");
    await pages[0].reload(); await wait(pages[0], () => document.body.dataset.playReady === "true");
    assert.equal(await pages[0].locator("body").getAttribute("data-phase"), "signed_out");
    for (const headers of socketHeaders) { assert.equal(headers.authorization, undefined); assert.equal(headers.cookie, undefined); }
    assert.deepEqual(errors, []);
    reports.push({ engine: name, renderer: launched.renderer, two_tabs: true, normal_tls: true, offset_ms: String(offset), cooldown_reconnect: true,
      movement: true, logout_revoked: true, transient_auth: true, text_200_percent: true });
    console.log(`PASS ${name}: deployed two-tab play, independent deadlines, reconnect, movement, logout and enlarged text`);
    await context.close();
  } finally { await launched.stop(); }
}
await writeFile(path.join(config.output, "play-proof.json"), JSON.stringify(reports, null, 2) + "\n");
