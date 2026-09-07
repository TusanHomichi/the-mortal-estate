#!/usr/bin/env node
// Reads real transport for assertions; every operation is performed through UI.
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import path from "node:path";
import { launchProofBrowser, PROOF_ENGINES } from "./serve.mjs";

let input = "";
for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input); input = "";
const engine = PROOF_ENGINES[config.engine];
if (!engine) throw new Error("Unrostered browser");
const launched = await launchProofBrowser({ name: config.engine, engine, executablePath: engine.executablePath(), trustedAuthority: config.authority });
try {
  const context = launched.context || await launched.browser.newContext({ viewport: { width: 1280, height: 900 } });
  const page = await context.newPage();
  const errors = [], commands = [], results = [];
  let frame;
  page.on("pageerror", error => errors.push(error.message));
  page.on("websocket", socket => {
    socket.on("framereceived", event => {
      const envelope = JSON.parse(String(event.payload));
      if (envelope.frame) frame = envelope.frame;
      if (envelope.kind === "command_result") results.push(envelope);
    });
    socket.on("framesent", event => {
      const envelope = JSON.parse(String(event.payload));
      if (envelope.kind === "command") commands.push(envelope);
    });
  });
  const wait = (predicate, argument) => page.waitForFunction(predicate, argument, { polling: 50, timeout: 30_000 });
  const ready = () => wait(() => document.querySelector("#world-canvas").dataset.canAct === "true" && document.querySelector("#world-canvas").dataset.pending === "false");
  async function login() {
    await page.locator("#username").fill(config.username);
    await page.locator("#password").fill(config.password);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();
    await wait(() => document.body.dataset.phase === "selecting");
    if (config.scenario === "town_adventure_loop_gallery") {
      await page.getByRole("button", { name: "Create a new character", exact: true }).click();
      await page.locator("#creation-form").waitFor({ state: "visible" });
      await page.locator("#creation-name").fill("New Arrival");
      await page.getByRole("button", { name: "Create character", exact: true }).click();
      await wait(() => document.querySelector("#character").selectedOptions[0]?.textContent === "New Arrival");
    }
    await page.getByRole("button", { name: "Enter world", exact: true }).click();
    await wait(() => document.body.dataset.phase === "playing");
    await ready();
  }
  function serviceActions(service) {
    return service.capabilities.flatMap(capability => {
      if (capability.actions) return capability.actions;
      if (capability.transactions) return capability.transactions.flatMap(row => row.actions);
      if (capability.operations) return capability.operations.flatMap(row => row.actions);
      if (capability.listings) return [...capability.listings.map(row => row.purchase), capability.buy_all, ...capability.sales];
      return [...capability.deposit_actions, ...capability.withdrawal_actions];
    });
  }
  async function action(predicate, { amount, service } = {}) {
    await ready();
    const groups = [...frame.services_here.map(row => ({ key: `service:${row.service_id}`, actions: serviceActions(row) })),
      { key: "character", actions: frame.action_options }];
    const group = groups.find(row => (!service || row.key === `service:${service}`) && row.actions.some(action => action.enabled && action.intent && predicate(action.intent)));
    if (!group) await writeFile(path.join(config.output, `${config.engine}-${config.scenario}-missing-action.json`), JSON.stringify(frame, null, 2));
    assert(group, "The real server did not offer the required action");
    const selected = group.actions.find(action => action.enabled && action.intent && predicate(action.intent));
    const section = page.locator(`details[data-group=${JSON.stringify(group.key)}]`);
    if (!await section.getAttribute("open").then(value => value !== null)) await section.locator("summary").click();
    await section.locator("select").selectOption(selected.id);
    if (amount) await section.locator("input").fill(amount);
    const count = commands.length;
    await section.getByRole("button", { name: "Perform selected action" }).click();
    await wait(() => document.querySelector("#world-canvas").dataset.pending === "false");
    assert.equal(commands.length, count + 1, "UI did not send exactly one command");
    const sent = commands.at(-1), result = results.find(row => row.command_id === sent.command_id);
    assert(result, "command receipt missing");
    assert.equal(result.disposition.kind, "accepted", `real rules refused ${sent.intent.kind}`);
    await wait(revision => {
      const observed = document.querySelector("#world-canvas").dataset.worldRevision;
      return observed !== "" && BigInt(observed) >= BigInt(revision);
    }, result.after_revision);
    await ready();
  }
  async function reconnect() {
    await page.getByRole("button", { name: "Reconnect", exact: true }).click();
    await wait(() => document.body.dataset.phase === "playing"); await ready();
  }
  await page.goto(config.origin);
  await wait(() => document.body.dataset.playReady === "true");
  await login();
  const observations = {};
  if (config.scenario === "gold_bank_locker_storage") {
    await action(intent => intent.kind === "move_gold" && intent.source.kind === "carried" && intent.source.position === "sack" && intent.destination.kind === "ground_here", { amount: "30" });
    await action(intent => intent.kind === "deposit_bank_gold", { service: "west_counter" });
    const balance = () => frame.services_here.flatMap(row => row.capabilities).filter(row => row.kind === "bank").map(row => row.balance_gold);
    assert.deepEqual(balance(), ["30", "30"]);
    await action(intent => intent.kind === "deposit_locker_item" && intent.item_instance_id === "field_case", { service: "west_counter" });
    assert(!frame.carried.items.some(row => row.item.item_instance_id === "field_case"));
    await reconnect(); assert.deepEqual(balance(), ["30", "30"]);
    await action(intent => intent.kind === "withdraw_locker_item" && intent.item_instance_id === "field_case" && intent.destination === "sack_item_1", { service: "east_counter" });
    await action(intent => intent.kind === "withdraw_bank_gold", { amount: "18", service: "east_counter" });
    assert.deepEqual(balance(), ["12", "12"]);
    await action(intent => intent.kind === "move_gold" && intent.source.kind === "ground" && intent.destination.kind === "carried" && intent.destination.position === "sack");
    assert.equal(frame.carried.gold.sack, "108");
    await reconnect(); assert.equal(frame.carried.gold.sack, "108"); assert.deepEqual(balance(), ["12", "12"]);
    assert(frame.carried.items.some(row => row.item.item_instance_id === "field_case"));
    observations.bank = "12"; observations.sack = "108"; observations.shared_locker_returned = true;
  } else if (config.scenario === "gold_training") {
    const before = structuredClone(frame.character);
    await action(intent => intent.kind === "train", { amount: "40", service: "sword_trainer" });
    const trained = structuredClone(frame.character);
    const sword = character => character.skill_ledger.find(row => row.track_id === "sword");
    const reward = results.at(-1).events.filter(row => row.kind === "feedback" && row.cue.kind === "transaction")
      .flatMap(row => row.cue.rewards).find(row => row.kind === "learning_rate" && row.track_id === "sword");
    assert(reward, "training receipt omitted its learning-rate change");
    assert(BigInt(reward.after) > BigInt(reward.before));
    assert.equal(sword(trained).learning_rate, reward.after);
    assert.equal(sword(trained).level, sword(before)?.level ?? 0);
    assert(BigInt(trained.progression.experience) > BigInt(before.progression.experience));
    assert(frame.gold_piles.some(row => BigInt(row.amount) > 0n), "excess training gold did not reach the ground");
    await reconnect(); assert.deepEqual(frame.character.skill_ledger, trained.skill_ledger);
    assert.equal(frame.carried.gold.sack, "460");
    assert.equal(frame.gold_piles.reduce((sum, row) => sum + BigInt(row.amount), 0n), 33n);
    await action(intent => intent.kind === "move_gold" && intent.source.kind === "ground" && intent.destination.kind === "carried" && intent.destination.position === "sack");
    assert.equal(frame.carried.gold.sack, "493");
    observations.training_persisted = true; observations.practice_still_required = true;
    observations.excess_returned_and_collected = true;
  } else {
    assert(frame.observer_actor_id.startsWith("created/"), "expedition must use the UI-created character");
    assert.equal(frame.character.identity.current_class_id, "wizard");
    observations.character_created_through_ui = true;
    await action(intent => intent.kind === "physical_attack" && intent.target_actor_id === "road_scavenger" && intent.mode === "fight");
    assert(frame.actors.some(row => row.actor_id === "road_scavenger" && row.life_state === "dead" && row.hp === 0));
    await action(intent => intent.kind === "search_corpse");
    await action(intent => intent.kind === "move_item" && intent.item_instance_id === "trade_charm" && intent.destination.kind === "carried" && intent.destination.position === "sack_item_2");
    await action(intent => intent.kind === "move_path" && intent.path.length === 1 && intent.path[0] === "east");
    if (frame.observation_center.level === "trailhead") await action(intent => intent.kind === "traverse" && intent.traversal === "stairs_up");
    assert.equal(frame.observation_center.level, "waystation");
    await action(intent => intent.kind === "buy_from_merchant" && intent.item_instance_ids.length === 1 && intent.item_instance_ids[0] === "bright_staff_stock");
    const bought = frame.carried.items.map(row => row.item.item_instance_id);
    assert(bought.includes("bright_staff_stock"));
    await reconnect(); assert(frame.carried.items.some(row => row.item.item_instance_id === "bright_staff_stock"));
    assert(frame.carried.items.some(row => row.item.item_instance_id === "trade_charm"));
    await action(intent => intent.kind === "traverse" && intent.traversal === "stairs_down");
    assert.equal(frame.observation_center.level, "trailhead");
    observations.encounter_loot_and_return = true; observations.purchase_persisted = true;
  }
  await page.locator("#settings summary").click();
  await page.locator("#text-scale").selectOption("200");
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), "200% text caused horizontal overflow");
  await page.locator("#text-scale").selectOption("100");
  await page.screenshot({ path: path.join(config.output, `${config.engine}-${config.scenario}.png`), fullPage: true });
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await wait(() => document.body.dataset.phase === "signed_out");
  assert.equal(await page.locator("#gameplay").textContent(), "");
  const storage = await page.evaluate(() => ({ local: Object.keys(localStorage), session: Object.keys(sessionStorage) }));
  assert.deepEqual(storage, { local: ["tme.play.preferences"], session: [] });
  assert.deepEqual(errors, []);
  await writeFile(path.join(config.output, `${config.engine}-${config.scenario}.json`), JSON.stringify({ verdict: "PASS",
    engine: config.engine, scenario: config.scenario, renderer: launched.renderer, commands: commands.map(row => row.intent.kind), observations,
    native_transport: true, normal_tls: true, scratch_postgres: true, credential_storage: storage }, null, 2));
  await context.close();
} finally { await launched.stop(); }
