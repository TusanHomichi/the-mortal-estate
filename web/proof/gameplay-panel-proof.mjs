#!/usr/bin/env node
// Native mouse gestures straddling authoritative refreshes. No gameplay transport.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { startVite, proofBrowsers, launchProofBrowser } from './serve.mjs';
const cases = JSON.parse(readFileSync(new URL('../../tests/fixtures/wire/server_envelope.json', import.meta.url))).cases;
const envelope = JSON.parse(cases.find(row => row.case_id === 'accept_server_welcome').input_utf8);
const vite = await startVite(process.env.TME_FEEL_ASSETS);
try {
  for (const spec of proofBrowsers()) {
    const launched = await launchProofBrowser(spec);
    try {
      const page = await launched.browser.newPage({ viewport: { width: 1280, height: 720 } });
      await page.setViewportSize({ width: 1280, height: 720 });
      await page.bringToFront();
      await page.route('**/panel-proof', route => route.fulfill({ contentType: 'text/html', body: '<!doctype html><div id="panel"></div>' }));
      await page.goto(`${vite.baseUrl}/panel-proof`);
      await page.evaluate(async envelope => {
        const { GameplayPanel } = await import('/src/play/gameplayPanel.ts');
        envelope.frame.services_here = []; envelope.frame.npcs_here = []; envelope.frame.can_act = true;
        envelope.frame.action_options = [{ id: 'gold', label: 'Move gold', enabled: true, blocked_reason: null,
          intent: { kind: 'move_gold', quantity: { kind: 'all' } } }];
        window.calls = [];
        window.view = { phase: 'playing', busy: false, pending: false, snapshot: { generation: 1, envelope } };
        window.panel = new GameplayPanel(document.querySelector('#panel'), (...args) => { window.calls.push(args); return true; });
        window.panel.present(window.view);
        document.querySelector('details').open = false;
      }, envelope);
      const summary = page.locator('summary');
      await summary.scrollIntoViewIfNeeded();
      const summaryBox = await summary.boundingBox();
      await page.mouse.move(summaryBox.x + 40, summaryBox.y + summaryBox.height / 2);
      await page.mouse.down();
      await page.evaluate(() => { ++window.view.snapshot.generation; window.panel.present(window.view); });
      await page.mouse.up();
      await page.waitForFunction(() => document.querySelector('details').open, undefined, { polling: 50, timeout: 5000 });
      const button = page.getByRole('button', { name: 'Perform selected action' });
      await page.locator('select').selectOption('gold');
      await page.locator('input').fill('123');
      await page.evaluate(() => { window.original = document.querySelector('button'); });
      await button.scrollIntoViewIfNeeded();
      const box = await button.boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.down();
      await page.evaluate(() => { ++window.view.snapshot.generation; window.panel.present(window.view); });
      await page.mouse.up();
      await page.waitForFunction(() => window.calls.length > 0, undefined, { polling: 50, timeout: 5000 });
      assert.deepEqual(await page.evaluate(() => window.calls), [[3, 'character', 'gold', '123']], `${spec.name}: refresh swallowed click or dispatched stale generation`);
      assert(await page.evaluate(() => window.original === document.querySelector('button')), 'retained action replaced its control');
      await page.locator('input').focus();
      await page.evaluate(() => { ++window.view.snapshot.generation; window.panel.present(window.view); });
      assert.equal(await page.locator('input').inputValue(), '123');
      assert(await page.locator('input').evaluate(node => node === document.activeElement), 'amount lost focus');
      // The same gesture must not dispatch if the new frame revokes the action.
      await page.mouse.down();
      await page.evaluate(() => { ++window.view.snapshot.generation; window.view.snapshot.envelope.frame.action_options[0].enabled = false; window.panel.present(window.view); });
      await page.mouse.up();
      assert.equal((await page.evaluate(() => window.calls)).length, 1);
      assert(await button.isDisabled());
      // Disconnect invalidates even a retained reference to the old control.
      await page.evaluate(() => { window.view.snapshot = null; window.panel.present(window.view); window.original.click(); });
      assert.equal((await page.evaluate(() => window.calls)).length, 1);
      assert.equal(await page.locator('#panel button').count(), 0);
      console.log(`PASS gameplay panel native refresh gesture: ${spec.name}`);
    } finally { await launched.stop(); }
  }
} finally { await vite.stop(); }
