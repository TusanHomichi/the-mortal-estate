#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { createCharacterManually } from "./creation-allocation.mjs";
import { launchProofBrowser, PROOF_ENGINES } from "./serve.mjs";
import { checkpointLedger, restartServingProcess } from "./proof-handshake.mjs";
let input = "";
for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input); input = "";
const engine = PROOF_ENGINES[config.engine];
const journey = config.journey ?? "success";
if (!engine) throw new Error("Unrostered engine");
const launched = await launchProofBrowser({ name: config.engine, engine, executablePath: engine.executablePath(), trustedAuthority: config.authority });
const geography = JSON.parse(await readFile(new URL("../../content/lands/first-expedition/generated/workbench_projection.json", import.meta.url), "utf8"));
let page, frame, staticContext, stage = "starting";
const errors = [], commands = [], results = [], checkpoints = [];
// A bounded window of authoritative frames. The latest frame is enough for
// most steps, but a transition like the return to life has to be observed as
// it happened, not reconstructed from whatever arrived last.
const frames = [];
try {
  const context = launched.context || await launched.browser.newContext({ viewport: { width: 1280, height: 1000 } });
  page = await context.newPage();
  page.on("pageerror", error => errors.push(error.message));
  page.on("websocket", socket => {
    socket.on("framereceived", event => { const envelope = JSON.parse(String(event.payload));
      if (envelope.frame) { frame = envelope.frame; staticContext = envelope.static_scene_context;
        frames.push(envelope.frame); if (frames.length > 400) frames.shift(); }
      if (envelope.kind === "command_result") results.push(envelope);
    });
    socket.on("framesent", event => { const e = JSON.parse(String(event.payload)); if (e.kind === "command") commands.push(e); });
  });
  const wait = (fn, arg, timeout = 45000) => page.waitForFunction(fn, arg, { polling: 50, timeout });
  // Node-side polling: some expectations are about the latest observed frame
  // rather than about the document, so they cannot run in the page context.
  const eventually = async (predicate, timeout = 30000) => {
    const end = Date.now() + timeout;
    while (!predicate()) { if (Date.now() > end) return false; await new Promise(resolve => setTimeout(resolve, 25)); }
    return true;
  };
  // The timeout is a parameter, not a decoration: a ghost becomes ready for its
  // own return only after the character's authored deadline, which is longer
  // than the default, and a call that silently used the default would time out
  // while the product was behaving correctly.
  const ready = (timeout = 45000) => wait(() => document.body.dataset.phase === "playing" && document.querySelector("#world-canvas").dataset.canAct === "true" && document.querySelector("#world-canvas").dataset.pending === "false", null, timeout);
  const here = () => frame.observation_center;
  // Collections differ in shape: carried items nest the instance under `item`,
  // while ground items, corpse contents and locker contents flatten it. One pair
  // of readers handles both, so a shape change cannot read as an empty list.
  const instanceOf = row => (row.item ?? row).item_instance_id;
  const definitionOf = row => (row.item ?? row).item_definition_id;
  async function mark(name) {
    stage = name; checkpoints.push({ stage, location: here(), gold: frame.carried.gold.sack, hp: frame.character.resources.hp,
      items: frame.carried.items.map(row => `${row.item.item_definition_id}@${row.position}`) });
    await writeFile(path.join(config.output, `${config.engine}-progress.json`), JSON.stringify({ stage, commands: commands.length, checkpoints }, null, 2));
    assert.equal(await page.locator("#world-canvas").getAttribute("data-study-level"), here().level);
    await page.locator("#world-canvas").screenshot({ path: path.join(config.output, `${config.engine}-${name}.png`) });
  }
  async function committed(count) {
    await wait(() => document.querySelector("#world-canvas").dataset.pending === "false");
    assert.equal(commands.length, count + 1);
    const result = results.find(row => row.command_id === commands.at(-1).command_id);
    assert(result, "missing command receipt");
    assert.equal(result.disposition.kind, "accepted", `refused ${commands.at(-1).intent.kind}`);
    await wait(revision => BigInt(document.querySelector("#world-canvas").dataset.worldRevision || "0") >= BigInt(revision), result.after_revision);
    await ready();
    return result;
  }
  const serviceActions = service => service.capabilities.flatMap(cap => cap.actions ??
    (cap.transactions ? cap.transactions.flatMap(row => row.actions) : cap.operations ? cap.operations.flatMap(row => row.actions) : cap.listings ? [...cap.listings.map(row => row.purchase), cap.buy_all, ...cap.sales] : [...cap.deposit_actions, ...cap.withdrawal_actions]));
  // The observed action list is recomputed by the server for the square the
  // character actually stands on. A frame can arrive immediately after a move
  // and still carry the previous square's offers, so an expected action is
  // waited for rather than demanded from whatever frame happened to be last.
  const groupsNow = () => [...frame.services_here.map(row => ({ key: `service:${row.service_id}`, actions: serviceActions(row) })), { key: "character", actions: frame.action_options }];
  const offeredIn = key => groupsNow().find(g => g.key === key);
  async function action(predicate, { service, amount } = {}) {
    await ready();
    const key = service ? `service:${service}` : null;
    await eventually(() => (key ? [offeredIn(key)] : groupsNow()).some(g => g && g.actions.some(a => a.enabled && a.intent && predicate(a.intent))));
    const groups = key ? [offeredIn(key)] : groupsNow();
    const group = groups.find(g => g && g.actions.some(a => a.enabled && a.intent && predicate(a.intent)));
    assert(group, `required action absent at ${JSON.stringify(here())} with ${JSON.stringify(groups.map(g => g.actions.length))}`);
    const chosen = group.actions.find(a => a.enabled && a.intent && predicate(a.intent));
    // The 3D canvas owns the viewport, so the action panel can be scrolled out
    // of the visible area at this window size. The controls are present and
    // enabled; dispatch them directly rather than depending on layout.
    const section = page.locator(`details[data-group=${JSON.stringify(group.key)}]`);
    if (await section.getAttribute("open") === null) {
      await section.locator("summary").evaluate(node => node.closest("details").open = true);
    }
    await section.locator("select").evaluate((node, value) => {
      node.value = value;
      node.dispatchEvent(new Event("change", { bubbles: true }));
    }, chosen.id);
    if (amount !== undefined) {
      await section.locator("input").evaluate((node, value) => { node.value = value; }, amount);
    }
    const count = commands.length;
    // Dispatch the panel's own activation handler: the button is the product's
    // real dispatch path, but its layout is not what this proof is testing.
    await section.locator("button", { hasText: "Perform selected action" })
      .evaluate(node => node.click());
    return committed(count);
  }
  const steps = [[0,-1,"North"],[1,0,"East"],[0,1,"South"],[-1,0,"West"]];
  // The world canvas owns the viewport, so the diagnostic direction column can
  // be laid out off-screen. Movement keys are the client's own input path and
  // are what a player uses; the proof presses those instead of hidden buttons.
  const binding = { North: "ArrowUp", East: "ArrowRight", South: "ArrowDown", West: "ArrowLeft" };
  async function walkTo(x, y) {
    await ready();
    const start = here(), key = p => `${p.x}:${p.y}`, target = `${x}:${y}`;
    const member = geography.members.find(m => m.member === start.level);
    const open = new Set(member.cells.filter(c => c.passable).map(key));
    for (const exit of geography.connectivity.edges.filter(e => e.from_member === start.level)) if (exit.direction === "passage" && key(exit.from) !== target) open.delete(key(exit.from));
    const queue = [{ ...start.position, route: [] }], seen = new Set([key(start.position)]);
    let route;
    for (let i = 0; i < queue.length; i++) {
      const p = queue[i]; if (key(p) === target) { route = p.route; break; }
      for (const [dx,dy,name] of steps) { const q = { x: p.x+dx, y: p.y+dy }; const k=key(q);
        if (open.has(k) && !seen.has(k)) { seen.add(k); queue.push({ ...q, route: [...p.route, name] }); }
      }
    }
    assert(route, `no observed map route to ${target}`);
    for (const direction of route) {
      await ready(); const count = commands.length;
      await page.locator("#world-canvas").focus();
      await page.keyboard.press(binding[direction]);
      await committed(count);
    }
    if (here().level === start.level) assert.deepEqual(here().position, { x, y });
  }
  async function enter(target) {
    const exit = geography.connectivity.edges.find(e => e.from_member === here().level && e.to_member === target);
    assert(exit, `missing connection to ${target}`);
    await walkTo(exit.from.x, exit.from.y);
    if (here().level !== target) await action(i => i.kind === "traverse" && i.traversal === (target === "d1_entry" ? "stairs_down" : "stairs_up"));
    assert.equal(here().level, target);
  }
  async function reconnectToOrigin() {
    await page.getByRole("button", { name: "Reconnect", exact: true }).click(); await ready();
  }
  const observer = () => frame.actors.find(row => row.actor_id === frame.observer_actor_id);
  const ghost = () => observer()?.life_state === "ghost";
  const heldWeapon = () => frame.carried.items.find(row => row.item.item_instance_id === createdWeaponId);
  const opponentRow = () => frame.actors.find(row => row.actor_id === "cellar_scavenger");
  const opponent = i => i.kind === "physical_attack" && i.target_actor_id === "cellar_scavenger";
  const opponentDefeated = () => frame.corpses.some(row => row.origin_actor_id === "cellar_scavenger");
  // The opponent holds ground and Fight reaches one square, so one real East
  // step is what joins the encounter. Every later action is an ordinary action.
  async function joinEncounter() {
    await ready(); const count = commands.length;
    await page.locator("#world-canvas").focus();
    await page.keyboard.press("ArrowRight");
    await committed(count);
  }
  /// Attack the shipped opponent until it dies, and snapshot the encounter the
  /// moment it ends. The snapshot is the encounter's own outcome: reading HP
  /// again after travel, rest and reconnect would report a later, healed state.
  async function fightToVictory(encounterLog) {
    const hpBefore = frame.character.resources.hp;
    let rounds = 0, damagingHits = 0, damageTaken = 0;
    for (let attempts = 0; attempts < 60 && !opponentDefeated(); attempts++) {
      // Fight reaches one square, which is where the previous step put the
      // character; the closing kick is the authored alternative when the offered
      // action list has no ready melee attack this round.
      const offered = () => frame.action_options.some(a => a.enabled && a.intent && opponent(a.intent) && a.intent.mode === "fight");
      await action(offered() ? (i => opponent(i) && i.mode === "fight") : (i => opponent(i) && i.mode === "jumpkick"));
      rounds += 1;
      damageTaken = hpBefore - frame.character.resources.hp;
      damagingHits = encounterLog.filter(row => row.player_hp_before > row.player_hp).length + 1;
      encounterLog.push({
        round: rounds, opponent_hp: frame.actors.find(row => row.actor_id === "cellar_scavenger")?.hp ?? null,
        player_hp: frame.character.resources.hp, player_hp_before: hpBefore,
        held: frame.carried.items.filter(row => row.position === "right_hand").map(row => row.item.item_definition_id),
        ground: frame.ground_items.map(definitionOf),
        corpse: frame.corpses.flatMap(row => (row.contents ?? []).map(definitionOf)),
      });
      await writeFile(path.join(config.output, `${config.engine}-encounter-log.json`), JSON.stringify(encounterLog, null, 2));
    }
    // Snapshot immediately, before any later step can heal or rest. Cumulative
    // damage taken, the net change across the exchange and any healing inside it
    // are three different numbers, so all three are recorded rather than one
    // standing in for the others.
    const hpAtEnd = frame.character.resources.hp;
    const snapshot = {
      rounds, damage_taken: damageTaken, damaging_hits: damagingHits,
      starting_hp: hpBefore, hp_at_end_of_combat: hpAtEnd,
      net_hp_change: hpBefore - hpAtEnd,
      healing_during_combat: damageTaken - (hpBefore - hpAtEnd),
      opponent_defeated: opponentDefeated(),
      corpse_present: frame.corpses.some(row => row.origin_actor_id === "cellar_scavenger"),
    };
    await writeFile(path.join(config.output, `${config.engine}-encounter-snapshot.json`), JSON.stringify(snapshot, null, 2));
    return { rounds, damageTaken, hpBefore, hpAfter: snapshot.hp_at_end_of_combat, defeated: snapshot.opponent_defeated, snapshot };
  }
  /// Recover the weapon ordinary creation put in the character's hand. A
  /// weapon-backed attack can fumble and drop it, which is ordinary authored
  /// combat: the opponent may then take it, and the character can take it back
  /// from the corpse. Recovered at the encounter site, where it is still visible.
  async function recoverCreatedWeapon() {
    for (let attempt = 0; attempt < 4 && heldWeapon()?.position !== "right_hand"; attempt++) {
      if (!heldWeapon()) {
        const onGround = frame.ground_items.find(row => row.item_instance_id === createdWeaponId);
        if (onGround) {
          await action(i => i.kind === "move_item" && i.item_instance_id === createdWeaponId && i.destination.kind === "carried");
          continue;
        }
        assert(frame.corpses.length > 0, "the created weapon is not on the ground and there is no corpse to search");
        await action(i => i.kind === "search_corpse");
      }
      await action(i => i.kind === "move_item" && i.item_instance_id === createdWeaponId
        && i.destination.kind === "carried");
    }
    if (heldWeapon() && heldWeapon().position !== "right_hand") {
      await action(i => i.kind === "move_item" && i.item_instance_id === createdWeaponId
        && i.destination.kind === "carried" && i.destination.position === "right_hand");
    }
    if (heldWeapon()?.position !== "right_hand") {
      await writeFile(path.join(config.output, `${config.engine}-weapon-recovery-failure.json`),
        JSON.stringify({ createdWeaponId, carried: frame.carried.items, ground: frame.ground_items,
          corpses: frame.corpses }, null, 2));
    }
    assert.equal(heldWeapon()?.position, "right_hand",
      "the created weapon must be recovered from the encounter before leaving it");
  }
  /// The ordinary death journey's second half: observe the ghost, prove the
  /// death state survives a serving-process restart, request the supported
  /// return once the character is eligible, and resume living movement. Each
  /// step is the product's own control or an ordinary command.
  async function completeOrdinaryDeath() {
    const character = frame.social.character_id;
    const corpse = structuredClone(frame.observation_center);
    const deadline = frame.ready_at;
    const items = structuredClone(frame.carried.items);
    const goldBefore = frame.carried.gold.sack;
    const hpAtDeath = frame.character.resources.hp;
    assert.equal(hpAtDeath, 0, "an ordinary death leaves the character at zero health");
    await mark("ghost");
    const speech = page.getByRole("form", { name: "Local speech", exact: true });
    assert(await speech.isVisible(), "the ghost retains local speech");
    assert(await page.getByRole("button", { name: "Request resurrection", exact: true }).isDisabled(),
      "the return must not be offered before the character's own deadline");
    // The eligibility threshold is a wait, not a failure to hide behind sleeps.
    await ready(90000);
    assert(BigInt(frame.logical_time) >= BigInt(deadline), "the deadline elapsed before eligibility was claimed");
    const restart = await restartServingProcess(config, {
      engine: config.engine, journey: "death", actor_id: frame.observer_actor_id,
      character_id: character, ready_at: deadline, location: corpse, life_state: "ghost" }, eventually);
    assert.equal(restart.payload_comparison, "parsed_json_minus_named_volatile_fields",
      "the restart receipt must name how it compared the checkpoint");
    assert.equal(restart.durable_payload_unchanged, true,
      "the durable checkpoint payload changed across the restart");
    assert.equal(restart.physical_action_refused_while_dead, true,
      "a restarted ghost was allowed to perform a physical action");
    // The return request below is this half's own proof of acceptance, so the
    // runner must not have consumed it: it asks only while the character is
    // still inside its deadline, where the only correct answer is a refusal.
    if (restart.return_attempted_while_ineligible)
      assert.equal(restart.return_refused_before_deadline, true,
        "an ineligible character was offered its return early");
    else
      assert.equal(restart.return_left_to_the_browser, true,
        "the runner consumed the eligible character's return");
    // Re-enter through the ordinary client on the restarted origin.
    await page.goto(config.origin + "/"); await wait(() => document.body.dataset.playReady === "true");
    await page.locator("#username").fill(config.username); await page.locator("#password").fill(config.password);
    await page.getByRole("button", { name: "Sign in", exact: true }).click();
    await wait(() => document.body.dataset.phase === "selecting");
    // The account owns the harness's enrolled character as well as the one this
    // journey created, and the roster defaults to the first. Selecting by ID is
    // how the restarted session re-enters the character under proof.
    await page.locator(`#character input[value="${character}"]`).check();
    await page.getByRole("button", { name: "Enter world", exact: true }).click(); await ready(90000);
    assert.equal(frame.social.character_id, character, "the restart changed the character identity");
    assert.equal(observer()?.life_state, "ghost", "the restart resurrected the character");
    assert.deepEqual(frame.observation_center, corpse, "the restart moved the corpse-bound ghost");
    assert.equal(frame.ready_at, deadline, "the restart reset the return eligibility deadline");
    assert(await page.getByRole("form", { name: "Local speech", exact: true }).isVisible(),
      "the restarted ghost lost its speech");
    await mark("restarted-ghost");
    const returnStart = commands.length;
    await page.getByRole("button", { name: "Request resurrection", exact: true }).click();
    await wait(() => document.querySelector("#world-canvas").dataset.lifeState === "alive");
    assert.equal(commands.length, returnStart + 1, "the return is one ordinary request");
    const observerId = frame.observer_actor_id;
    const returnedFrame = frames.filter(value =>
      value.actors.some(row => row.actor_id === observerId && row.life_state === "alive"));
    assert(returnedFrame.length, "no authoritative frame reported the return");
    const returning = returnedFrame.at(-1);
    assert.deepEqual(returning.observation_center, config.destination, "the return landed at the configured destination");
    // Conservation, not a happy ending. Production scavenging stays enabled, so
    // the scavenging opponent legitimately strips the corpse and what it took is
    // expected to stay with it. What the return may not do is invent or destroy
    // anything; the authoritative audit of the stored checkpoint, run by the
    // runner over the capture taken before the encounter, is what decides that.
    // The count comparison that used to live here accepted an emptied inventory
    // and an emptied purse, so it survives only as a diagnostic in the receipt.
    const returnedIds = new Set(returning.carried.items.map(row => row.item.item_instance_id));
    assert(returning.carried.items.length <= items.length, "a return cannot create items");
    assert(BigInt(returning.carried.gold.sack) <= BigInt(goldBefore), "a return cannot create gold");
    await mark("returned");
    await page.locator("#world-canvas").focus();
    const beforeMove = commands.length;
    await page.keyboard.press("ArrowUp");
    await eventually(() => commands.length > beforeMove && JSON.stringify(frame.observation_center) !== JSON.stringify(config.destination));
    await ready();
    assert.notDeepEqual(frame.observation_center, config.destination, "the returned character can move again");
    await page.getByRole("button", { name: "Sign out", exact: true }).click();
    await wait(() => document.body.dataset.phase === "signed_out");
    assert.deepEqual(errors, []);
    return {
      restart_while_dead: restart, death_hp: hpAtDeath, death_deadline: deadline, corpse,
      return_destination: config.destination, items_at_creation: items,
      items_after_return: returning.carried.items, gold_at_creation: goldBefore,
      return_carried_gold: returning.carried.gold.sack,
      scavenging_enabled: true, ordinary_creation: true,
    };
  }
  /// Take ordinary Wait actions until the authored opponent removes the
  /// character. No HP is assigned and no event is synthesized: this is
  /// production combat resolving an ordinary death. Every round is kept, with
  /// both actors' positions, because an opponent that never acts is a
  /// positioning fact and the trace is what names it.
  async function dieToAuthoredOpponent() {
    const hpBefore = frame.character.resources.hp;
    const trace = [];
    let rounds = 0;
    await page.locator("#world-canvas").focus();
    while (!ghost() && rounds < 200) {
      await page.waitForFunction(() => {
        const canvas = document.querySelector("#world-canvas");
        return canvas.dataset.canAct === "true" || canvas.dataset.lifeState === "ghost";
      }, null, { polling: 30, timeout: 120000 });
      if (ghost()) break;
      const before = commands.length;
      await page.keyboard.press("Space");
      await eventually(() => commands.length > before || ghost(), 60000);
      await wait(() => document.querySelector("#world-canvas").dataset.pending === "false");
      rounds += 1;
      trace.push({
        round: rounds, player_hp: frame.character.resources.hp,
        opponent_hp: opponentRow()?.hp ?? null, life_state: observer()?.life_state ?? null,
        here: here(), opponent: opponentRow()?.position ?? null,
        command: commands.at(-1)?.intent ?? null,
      });
      await writeFile(path.join(config.output, `${config.engine}-death-journey-log.json`),
        JSON.stringify({ starting_hp: hpBefore, rounds, trace }, null, 2));
    }
    assert(ghost(), "the shipped opponent could not defeat a passive created character");
    const damageTaken = hpBefore - frame.character.resources.hp;
    assert(damageTaken > 0, "the character lost no health before dying");
    // The encounter boundary for the losing journey: what the exchange cost,
    // recorded where it ended rather than after the later return.
    const snapshot = {
      rounds_survived: rounds, damage_taken: damageTaken, starting_hp: hpBefore,
      hp_at_end_of_combat: frame.character.resources.hp,
      opponent_hp_at_end_of_combat: frame.actors.find(row => row.actor_id === "cellar_scavenger")?.hp ?? null,
      opponent_defeated: false, life_state: observer()?.life_state ?? null,
    };
    await writeFile(path.join(config.output, `${config.engine}-encounter-snapshot.json`),
      JSON.stringify(snapshot, null, 2));
    return snapshot;
  }
  await page.goto(`${config.origin}/play.html?study=first-expedition`);
  await wait(() => document.body.dataset.playReady === "true");
  await page.locator("#username").fill(config.username); await page.locator("#password").fill(config.password);
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await wait(() => document.body.dataset.phase === "selecting");
  await createCharacterManually(page, { className: "Wizard", name: "Expedition Arrival" });
  await wait(() => document.querySelector("#character input:checked")?.getAttribute("aria-label") === "Expedition Arrival");
  await page.getByRole("button", { name: "Enter world", exact: true }).click(); await ready();
  assert(frame.observer_actor_id.startsWith("created/"));
  // The weapon ordinary creation put in the character's hand, remembered so the
  // instruction step can require it to be held again after the encounter.
  const createdWeaponId=frame.carried.items.find(row=>row.position==="right_hand")?.item.item_instance_id ?? null;
  // The authored dock arrival, not the seeded occupant's historic position.
  assert.deepEqual(here().position, { x:8,y:34 });
  await mark("dock");
  // Which composed journey this run is. Both start from a character created
  // through the ordinary UI on the unmodified release world; they differ in what
  // the character does when it reaches the shipped opponent.
  if (journey === "death") {
    // The direct route: the bank and the provisioner belong to the success
    // route, and stopping at them would spend actions the doomed character does
    // not need. Everything from the temple onwards is the same navigation.
    await enter("temple"); await enter("d1_entry");
    assert.deepEqual(here().position, { x: 24, y: 7 }); await mark("descent");
    await walkTo(23, 9);
    // The authored opponent holds the square it is seeded on, and its reach is
    // that square: a character standing next to it and waiting is never attacked,
    // which is an observation this proof made before it made an assertion. The
    // death route therefore stands where the opponent stands, exactly as the
    // ordinary death-return fixture does.
    assert.deepEqual(here().position, opponentRow().position.position,
      "the death route must stand on the authored opponent's square");
    // The authoritative item and coin ledger, read from the stored checkpoint by
    // the runner while the created character is still alive. From here the world
    // may move an instance into the corpse, onto the ground or into the
    // scavenging opponent's hands, but nothing may stop existing or appear.
    const ledgerBefore = await checkpointLedger(config, "capture", "before-death", eventually);
    assert.equal(ledgerBefore.verdict, "captured", JSON.stringify(ledgerBefore));
    const deathEncounter = await dieToAuthoredOpponent();
    const deathEvidence = await completeOrdinaryDeath();
    // Conservation is decided by the runner's audit of the authoritative
    // checkpoint, not by counting what the returned character can see. A count
    // comparison accepts an emptied inventory, a same-count substitution and a
    // duplicate instance; this fails on each of them.
    const ledgerAfter = await checkpointLedger(config, "audit", "before-death", eventually);
    assert.equal(ledgerAfter.verdict, "audited", JSON.stringify(ledgerAfter.defects));
    assert.deepEqual(ledgerAfter.defects, [], "the death boundary must conserve every item and coin");
    assert.equal(ledgerAfter.after.item_count, ledgerAfter.before.item_count, "every instance still exists");
    assert.equal(ledgerAfter.after.gold_total, ledgerAfter.before.gold_total, "every coin still exists");
    assert.deepEqual(ledgerAfter.after.unowned, [], "every instance has exactly one owner");
    const destination = config.destination;
    await writeFile(path.join(config.output, `${config.engine}-expedition.json`), JSON.stringify({
      verdict: "PASS", engine: config.engine, journey: "death", renderer: launched.renderer,
      source: config.source,
      checkpoints, commands: commands.map(command => command.intent.kind),
      created_through_ui: true, authored_world: true, normal_tls: true, scratch_postgres: true,
      candidate_art: true, return_destination: destination,
      production_encounter: deathEncounter,
      ledger_before: ledgerBefore, ledger_after: ledgerAfter, ...deathEvidence }, null, 2));
    await context.close();
  } else {
  await enter("bank"); await walkTo(1,2);
  await action(i => i.kind === "move_gold" && i.source.kind === "carried" && i.source.position === "sack" && i.destination.kind === "ground_here", { amount: "20" });
  await action(i => i.kind === "deposit_bank_gold", { service: "banker" });
  const bank = () => frame.services_here.flatMap(s => s.capabilities).find(c => c.kind === "bank").balance_gold;
  assert.equal(bank(), "20"); await reconnectToOrigin(); assert.equal(bank(), "20");
  await mark("bank"); await enter("arrival"); await enter("temple");
  const seller = frame.actors.find(actor => actor.actor_id === "balm_seller").position.position;
  await walkTo(seller.x,seller.y); await action(i => i.kind === "buy_from_merchant" && i.item_instance_ids.length === 1, { service: "balm_seller" });
  assert(frame.carried.items.some(i => i.item.item_definition_id === "healing_balm"));
  await mark("temple"); await enter("d1_entry");
  assert.deepEqual(here().position, { x:24,y:7 }); await mark("descent");
  await walkTo(23,9);
  await joinEncounter();
  // The encounter is the shipped opponent at its shipped ratings. Record what
  // the exchange actually cost: a block animation or a fast kill would not show
  // that the opponent is dangerous, and an instant loss would not show that it
  // is survivable.
  const encounterLog = [];
  const { rounds, damageTaken, hpBefore, hpAfter, defeated, snapshot: encounterSnapshot } =
    await fightToVictory(encounterLog);
  assert(defeated, "the opponent survived the fight");
  assert(rounds > 1, `the encounter ended in ${rounds} round(s); the opponent never acted`);
  assert(damageTaken > 0, "the shipped opponent never injured the character");
  assert(hpAfter > 0, "the character did not survive its own starting encounter");
  assert(frame.corpses.some(row => row.origin_actor_id === "cellar_scavenger"), "the corpse is not the encountered opponent");
  await action(i => i.kind === "search_corpse");
  await action(i => i.kind === "move_item" && i.item_instance_id === "found_charm" && i.destination.kind === "carried" && i.destination.position === "sack_item_3");
  await action(i => i.kind === "move_gold" && i.source.kind === "ground" && i.destination.kind === "carried" && i.destination.position === "sack");
  await recoverCreatedWeapon();
  assert.equal(encounterSnapshot.hp_at_end_of_combat, hpAfter,
    "the reported combat outcome is the one measured at the encounter boundary");
  await mark("loot"); await enter("temple"); await enter("arrival"); await enter("trainers"); await walkTo(1,2);
  // The instructor's offered track is selected from what the character holds,
  // and a magic-capable character with a free hand is read as asking for spell
  // instruction and told to produce a bound spell book. The weapon was recovered
  // at the encounter site, so it is still in hand here.
  assert.equal(heldWeapon()?.position, "right_hand", "the created weapon must still be in hand at the instructor");
  const before = frame.character.skill_ledger.find(s => s.track_id === "staff").learning_rate;
  await action(i => i.kind === "train", { service: "trainer_1", amount: "14" });
  assert(BigInt(frame.character.skill_ledger.find(s => s.track_id === "staff").learning_rate) > BigInt(before));
  const critique = await action(i => i.kind === "critique" && i.track_id === "staff", { service: "trainer_1" });
  assert(critique.events.some(e => e.kind === "feedback" && e.cue.kind === "skill_critique"));
  assert.match(await page.locator("#feedback").textContent(), /Staff:/);
  const saved = { skills: frame.character.skill_ledger, items: frame.carried.items, gold: frame.carried.gold.sack, location: here() };
  await reconnectToOrigin(); assert.deepEqual(frame.character.skill_ledger,saved.skills); assert.deepEqual(frame.carried.items,saved.items); assert.equal(frame.carried.gold.sack,saved.gold); assert.deepEqual(here(),saved.location);
  await mark("trained-return");
  await enter("arrival"); await enter("lodge"); await walkTo(6,1);
  await action(i => i.kind === "deposit_locker_item" && i.item_instance_id === "found_charm", { service: "lodge_keeper" });
  assert(!frame.carried.items.some(i => instanceOf(i) === "found_charm"));
  await reconnectToOrigin();
  assert(frame.services_here.flatMap(s => s.capabilities)
    .some(c => c.kind === "locker" && c.items.some(i => instanceOf(i) === "found_charm")),
    "the locker must hold the deposited charm after a reconnect");
  await action(i => i.kind === "withdraw_locker_item" && i.item_instance_id === "found_charm" && i.destination === "sack_item_3", { service: "lodge_keeper" });
  assert(frame.carried.items.some(i => instanceOf(i) === "found_charm"));
  await mark("lodge-return");
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await wait(() => document.body.dataset.phase === "signed_out");
  assert.equal(await page.locator("#world-canvas").getAttribute("data-study-level"), null);
  assert.deepEqual(errors, []);
  await writeFile(path.join(config.output, `${config.engine}-expedition.json`), JSON.stringify({ verdict:"PASS", engine:config.engine, journey:"success", renderer:launched.renderer, source:config.source, checkpoints, commands:commands.map(c=>c.intent.kind), created_through_ui:true, authored_world:true, normal_tls:true, scratch_postgres:true, reconnect_preserved_state:true, candidate_art:true,
    encounter_rounds_logged:encounterLog.length,
    // The encounter figures are the ones frozen when combat ended, and the
    // receipt also says what later recovery did to the character, so a healed
    // later state can never be read as the outcome of the fight.
    production_encounter:{...encounterSnapshot, opponent_hp_authored:18, player_ratings_authored:true,
      hp_at_receipt:frame.character.resources.hp,
      post_combat_recovery:frame.character.resources.hp-encounterSnapshot.hp_at_end_of_combat} },null,2));
  // The per-round trace is useful evidence but does not belong in the summary
  // receipt; it is written beside it.
  await writeFile(path.join(config.output, `${config.engine}-encounter-log.json`), JSON.stringify(encounterLog, null, 2));
  await context.close();
  }
} catch(error) {
  await writeFile(path.join(config.output, `${config.engine}-failure.json`), JSON.stringify({ stage, error:String(error), errors, frame, staticContext, commandCount:commands.length, lastResult:results.at(-1) },null,2));
  await page?.screenshot({ path:path.join(config.output, `${config.engine}-failure.png`), fullPage:true }).catch(()=>{});
  throw error;
} finally { await launched.stop(); }
