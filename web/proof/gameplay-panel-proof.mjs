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
      // The actor menu holds the same discipline: unchanged offers keep their
      // control across a generation refresh, retired or detached ones cannot act.
      await page.evaluate(async envelope => {
        const { ActorInteraction } = await import('/src/play/actorInteraction.ts');
        const actorEnvelope = structuredClone(envelope);
        const service = actorEnvelope.frame.services_here.find(row => row.service_id === 'town_services');
        service.actor_id = 'actor_2';
        service.capabilities = service.capabilities.filter(capability => capability.kind === 'merchant');
        window.actorCalls = []; window.actorServices = structuredClone(actorEnvelope.frame.services_here);
        window.actorView = { phase: 'playing', busy: false, pending: false, snapshot: { generation: 1, envelope: actorEnvelope } };
        window.actors = new ActorInteraction((generation, group, action) => { window.actorCalls.push([generation, group, action]); return true; });
        window.actors.open('actor_2', window.actorView);
        // Pin the menu inside the viewport so the native gesture below targets
        // determined coordinates in every engine.
        document.querySelector('dialog.resident-dialog').style.cssText = 'position: fixed; inset: 0 auto auto 0; margin: 0;';
      }, envelope);
      const buy = page.locator('[data-action="service:town_services/buy_one"]');
      const buyCentre = async () => {
        await buy.scrollIntoViewIfNeeded();
        const box = await buy.boundingBox();
        return [box.x + box.width / 2, box.y + box.height / 2];
      };
      await page.evaluate(() => { window.actorFirst = document.querySelector('[data-action="service:town_services/buy_one"]'); });
      let [buyX, buyY] = await buyCentre();
      await page.mouse.move(buyX, buyY);
      await page.mouse.down();
      await page.evaluate(() => { ++window.actorView.snapshot.generation; window.actors.present(window.actorView); });
      assert(await page.evaluate(() => window.actorFirst === document.querySelector('[data-action="service:town_services/buy_one"]')),
        `${spec.name}: unchanged actor menu replaced its control across a refresh`);
      await page.mouse.up();
      assert.deepEqual(await page.evaluate(() => window.actorCalls), [[2, 'service:town_services', 'buy_one']],
        `${spec.name}: actor menu swallowed the native click or dispatched a stale generation`);
      await buy.focus();
      await page.evaluate(() => { ++window.actorView.snapshot.generation; window.actors.present(window.actorView); });
      assert(await page.evaluate(() => window.actorFirst === document.activeElement),
        `${spec.name}: actor menu lost keyboard focus across an unchanged refresh`);
      // A renamed actor updates the title without rebuilding the offered controls.
      await page.evaluate(() => {
        window.actorView.snapshot.generation += 1;
        window.actorView.snapshot.envelope.frame.actors.find(actor => actor.actor_id === 'actor_2').name = 'Registrar';
        window.actors.present(window.actorView);
      });
      assert.equal(await page.locator('#resident-title').textContent(), 'Registrar');
      assert(await page.evaluate(() => window.actorFirst === document.activeElement),
        `${spec.name}: title update replaced or unfocused the actor control`);
      // A changed merchant label rebuilds the control and restores focus by action identity.
      await page.evaluate(() => {
        window.actorView.snapshot.generation += 1;
        window.actorView.snapshot.envelope.frame.services_here
          .find(row => row.service_id === 'town_services').capabilities
          .find(capability => capability.kind === 'merchant').listings[0].item.name = 'Folded Deed';
        window.actors.present(window.actorView);
      });
      assert.equal(await buy.textContent(), 'Buy Folded Deed × 1 — 5 gold', `${spec.name}: merchant label did not follow its offer`);
      assert(await page.evaluate(() => window.actorFirst !== document.querySelector('[data-action="service:town_services/buy_one"]')),
        `${spec.name}: relabelled offer kept a stale control`);
      assert(await page.evaluate(() => document.activeElement.dataset.action === 'service:town_services/buy_one'),
        `${spec.name}: rebuilt actor menu lost focus identity`);
      await page.evaluate(() => { window.actorRevoked = document.querySelector('[data-action="service:town_services/buy_one"]'); });
      [buyX, buyY] = await buyCentre();
      await page.mouse.move(buyX, buyY);
      await page.mouse.down();
      await page.evaluate(() => {
        window.actorView.snapshot.generation += 1;
        const purchase = window.actorView.snapshot.envelope.frame.services_here
          .find(row => row.service_id === 'town_services').capabilities
          .find(capability => capability.kind === 'merchant').listings.find(row => row.purchase.id === 'buy_one').purchase;
        purchase.enabled = false; purchase.blocked_reason = 'refused';
        window.actors.present(window.actorView);
      });
      await page.mouse.up();
      assert.equal((await page.evaluate(() => window.actorCalls)).length, 1, `${spec.name}: revoked actor offer dispatched a native click`);
      assert(await buy.isDisabled());
      assert(await page.evaluate(() => !window.actorRevoked.isConnected), `${spec.name}: revoked actor offer kept its old control connected`);
      assert.equal(await page.evaluate(() => { window.actorRevoked.click(); return window.actorCalls.length; }), 1,
        `${spec.name}: revoked actor offer dispatched from a captured old control`);
      // Re-enable the offer so the dismissed and detached cases test a control that could otherwise act.
      await page.evaluate(() => {
        window.actorView.snapshot.generation += 1;
        const purchase = window.actorView.snapshot.envelope.frame.services_here
          .find(row => row.service_id === 'town_services').capabilities
          .find(capability => capability.kind === 'merchant').listings.find(row => row.purchase.id === 'buy_one').purchase;
        purchase.enabled = true; purchase.blocked_reason = null;
        window.actors.present(window.actorView);
        window.actorDismissed = document.querySelector('[data-action="service:town_services/buy_one"]');
      });
      assert(!(await buy.isDisabled()), `${spec.name}: actor offer did not become available again`);
      await page.locator('dialog.resident-dialog button:not([data-action])').click();
      assert.equal(await page.evaluate(() => document.querySelector('dialog.resident-dialog').open), false, `${spec.name}: actor menu refused to close`);
      assert.equal(await page.evaluate(() => { window.actorDismissed.click(); return window.actorCalls.length; }), 1,
        `${spec.name}: dismissed actor menu dispatched from an old control`);
      // Detached menus cannot dispatch either: the retained node must be refused.
      await page.evaluate(() => {
        window.actors.open('actor_2', window.actorView);
        window.actorDetached = document.querySelector('[data-action="service:town_services/buy_one"]');
      });
      await page.evaluate(() => {
        window.actorView.snapshot.generation += 1;
        window.actorView.snapshot.envelope.frame.services_here = [];
        window.actors.present(window.actorView);
      });
      assert(await page.evaluate(() => !window.actorDetached.isConnected), `${spec.name}: actor menu kept a detached control alive`);
      assert.equal(await page.locator('dialog.resident-dialog [data-action]').count(), 0);
      assert.equal(await page.evaluate(() => { window.actorDetached.click(); return window.actorCalls.length; }), 1,
        `${spec.name}: detached actor menu dispatched from an old control`);
      // Loss of the actual snapshot closes an otherwise enabled, connected menu.
      await page.evaluate(() => {
        ++window.actorView.snapshot.generation;
        window.actorView.snapshot.envelope.frame.services_here = window.actorServices;
        window.actors.open('actor_2', window.actorView);
        window.actorDisconnected = document.querySelector('[data-action="service:town_services/buy_one"]');
      });
      assert(!(await buy.isDisabled()));
      await page.evaluate(() => { window.actorView.snapshot = null; window.actors.present(window.actorView); });
      assert.equal(await page.evaluate(() => document.querySelector('dialog.resident-dialog').open), false);
      assert.equal(await page.evaluate(() => { window.actorDisconnected.click(); return window.actorCalls.length; }), 1,
        `${spec.name}: disconnected actor menu dispatched from an old control`);
      console.log(`PASS actor interaction native refresh gesture: ${spec.name}`);
    } finally { await launched.stop(); }
  }
} finally { await vite.stop(); }
