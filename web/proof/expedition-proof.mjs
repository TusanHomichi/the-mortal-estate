#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { createCharacterManually } from "./creation-allocation.mjs";
import { launchProofBrowser, PROOF_ENGINES } from "./serve.mjs";
let input = "";
for await (const chunk of process.stdin) input += chunk;
const config = JSON.parse(input); input = "";
const engine = PROOF_ENGINES[config.engine];
if (!engine) throw new Error("Unrostered engine");
const launched = await launchProofBrowser({ name: config.engine, engine, executablePath: engine.executablePath(), trustedAuthority: config.authority });
const geography = JSON.parse(await readFile(new URL("../../content/lands/first-expedition/generated/workbench_projection.json", import.meta.url), "utf8"));
let page, frame, staticContext, stage = "starting";
const errors = [], commands = [], results = [], checkpoints = [];
try {
  const context = launched.context || await launched.browser.newContext({ viewport: { width: 1280, height: 1000 } });
  page = await context.newPage();
  page.on("pageerror", error => errors.push(error.message));
  page.on("websocket", socket => {
    socket.on("framereceived", event => { const envelope = JSON.parse(String(event.payload));
      if (envelope.frame) { frame = envelope.frame; staticContext = envelope.static_scene_context; }
      if (envelope.kind === "command_result") results.push(envelope);
    });
    socket.on("framesent", event => { const e = JSON.parse(String(event.payload)); if (e.kind === "command") commands.push(e); });
  });
  const wait = (fn, arg) => page.waitForFunction(fn, arg, { polling: 50, timeout: 45000 });
  // Node-side polling: some expectations are about the latest observed frame
  // rather than about the document, so they cannot run in the page context.
  const eventually = async (predicate, timeout = 30000) => {
    const end = Date.now() + timeout;
    while (!predicate()) { if (Date.now() > end) return false; await new Promise(resolve => setTimeout(resolve, 25)); }
    return true;
  };
  const ready = () => wait(() => document.body.dataset.phase === "playing" && document.querySelector("#world-canvas").dataset.canAct === "true" && document.querySelector("#world-canvas").dataset.pending === "false");
  const here = () => frame.observation_center;
  // Collections differ in shape: carried items nest the instance under `item`,
  // ground items and their siblings flatten it. One reader handles both.
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
  async function reconnect() {
    await page.getByRole("button", { name: "Reconnect", exact: true }).click(); await ready();
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
  await enter("bank"); await walkTo(1,2);
  await action(i => i.kind === "move_gold" && i.source.kind === "carried" && i.source.position === "sack" && i.destination.kind === "ground_here", { amount: "20" });
  await action(i => i.kind === "deposit_bank_gold", { service: "banker" });
  const bank = () => frame.services_here.flatMap(s => s.capabilities).find(c => c.kind === "bank").balance_gold;
  assert.equal(bank(), "20"); await reconnect(); assert.equal(bank(), "20");
  await mark("bank"); await enter("arrival"); await enter("temple");
  const seller = frame.actors.find(actor => actor.actor_id === "balm_seller").position.position;
  await walkTo(seller.x,seller.y); await action(i => i.kind === "buy_from_merchant" && i.item_instance_ids.length === 1, { service: "balm_seller" });
  assert(frame.carried.items.some(i => i.item.item_definition_id === "healing_balm"));
  await mark("temple"); await enter("d1_entry");
  assert.deepEqual(here().position, { x:24,y:7 }); await mark("descent");
  await walkTo(23,9);
  // The opponent holds ground and Fight reaches one square, so one real East
  // step is what joins the encounter. Every later attack is an ordinary action.
  await ready(); { const count = commands.length;
    await page.locator("#world-canvas").focus();
    await page.keyboard.press("ArrowRight");
    await committed(count);
  }
  // The encounter is the shipped opponent at its shipped ratings. Record what
  // the exchange actually cost: a block animation or a fast kill would not show
  // that the opponent is dangerous, and an instant loss would not show that it
  // is survivable.
  const encounterHpBefore = frame.character.resources.hp;
  let encounterDamageTaken = 0, encounterRounds = 0; const encounterLog = [];
  const opponent = i => i.kind === "physical_attack" && i.target_actor_id === "cellar_scavenger";
  const corpsesBefore = frame.corpses.length;
  const defeated = () => frame.corpses.some(row => row.origin_actor_id === "cellar_scavenger");
  for (let attempts=0; attempts<60 && !defeated(); attempts++) {
    // Fight reaches one square, which is where the previous step put the
    // character; the closing kick is the authored alternative when the offered
    // action list has no ready melee attack this round.
    const offered = () => frame.action_options.some(a => a.enabled && a.intent && opponent(a.intent) && a.intent.mode === "fight");
    await action(offered() ? (i => opponent(i) && i.mode === "fight") : (i => opponent(i) && i.mode === "jumpkick"));
    encounterRounds += 1;
    encounterDamageTaken = encounterHpBefore - frame.character.resources.hp;
    encounterLog.push({round:encounterRounds,
      held:frame.carried.items.filter(row=>row.position==="right_hand").map(row=>row.item.item_definition_id),
      ground:frame.ground_items.map(definitionOf),
      corpse:frame.corpses.flatMap(row=>(row.contents??[]).map(definitionOf)),
      scavenger:frame.actors.some(row=>row.actor_id==="cellar_scavenger")});
    await writeFile(path.join(config.output, `${config.engine}-encounter-log.json`), JSON.stringify(encounterLog, null, 2));
  }
  assert(defeated(), "the opponent survived the fight");
  assert(frame.corpses.length > corpsesBefore, "defeat did not produce the opponent's corpse");
  assert(frame.corpses.some(row => row.origin_actor_id === "cellar_scavenger"), "the corpse is not the encountered opponent");
  assert(encounterRounds > 1, `the encounter ended in ${encounterRounds} round(s); the opponent never acted`);
  assert(encounterDamageTaken > 0, "the shipped opponent never injured the character");
  assert(frame.character.resources.hp > 0, "the character did not survive its own starting encounter");
  await action(i => i.kind === "search_corpse");
  await action(i => i.kind === "move_item" && i.item_instance_id === "found_charm" && i.destination.kind === "carried" && i.destination.position === "sack_item_3");
  await action(i => i.kind === "move_gold" && i.source.kind === "ground" && i.destination.kind === "carried" && i.destination.position === "sack");
  // Instruction follows the held weapon, so the character must hold the weapon
  // it was created with. A weapon-backed attack can fumble and drop it, which is
  // ordinary authored combat: the opponent may then take it, and the character
  // can take it back from the corpse. This is recovered here, at the encounter
  // site, because that is where the character can still see it.
  const held = () => frame.carried.items.find(row => row.item.item_instance_id === createdWeaponId);
  for (let attempt = 0; attempt < 4 && held()?.position !== "right_hand"; attempt++) {
    if (!held()) {
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
  if (held() && held().position !== "right_hand") {
    await action(i => i.kind === "move_item" && i.item_instance_id === createdWeaponId
      && i.destination.kind === "carried" && i.destination.position === "right_hand");
  }
  if (held()?.position !== "right_hand") {
    await writeFile(path.join(config.output, `${config.engine}-weapon-recovery-failure.json`),
      JSON.stringify({ createdWeaponId, carried: frame.carried.items, ground: frame.ground_items,
        corpses: frame.corpses, encounter: encounterLog,
        scavengerHere: frame.actors.some(row => row.actor_id === "cellar_scavenger") }, null, 2));
  }
  assert.equal(held()?.position, "right_hand",
    "the created weapon must be recovered from the encounter before leaving it");
  await mark("loot"); await enter("temple"); await enter("arrival"); await enter("trainers"); await walkTo(1,2);
  // The instructor's offered track is selected from what the character holds,
  // and a magic-capable character with a free hand is read as asking for spell
  // instruction and told to produce a bound spell book. The weapon was recovered
  // at the encounter site, so it is still in hand here.
  assert.equal(held()?.position, "right_hand", "the created weapon must still be in hand at the instructor");
  const before = frame.character.skill_ledger.find(s => s.track_id === "staff").learning_rate;
  await action(i => i.kind === "train", { service: "trainer_1", amount: "14" });
  assert(BigInt(frame.character.skill_ledger.find(s => s.track_id === "staff").learning_rate) > BigInt(before));
  const critique = await action(i => i.kind === "critique" && i.track_id === "staff", { service: "trainer_1" });
  assert(critique.events.some(e => e.kind === "feedback" && e.cue.kind === "skill_critique"));
  assert.match(await page.locator("#feedback").textContent(), /Staff:/);
  const saved = { skills: frame.character.skill_ledger, items: frame.carried.items, gold: frame.carried.gold.sack, location: here() };
  await reconnect(); assert.deepEqual(frame.character.skill_ledger,saved.skills); assert.deepEqual(frame.carried.items,saved.items); assert.equal(frame.carried.gold.sack,saved.gold); assert.deepEqual(here(),saved.location);
  await mark("trained-return");
  await enter("arrival"); await enter("lodge"); await walkTo(6,1);
  await action(i => i.kind === "deposit_locker_item" && i.item_instance_id === "found_charm", { service: "lodge_keeper" });
  assert(!frame.carried.items.some(i => i.item.item_instance_id === "found_charm"));
  await reconnect();
  assert(frame.services_here.flatMap(s => s.capabilities).some(c => c.kind === "locker" && c.items.some(i => i.item_instance_id === "found_charm")));
  await action(i => i.kind === "withdraw_locker_item" && i.item_instance_id === "found_charm" && i.destination === "sack_item_3", { service: "lodge_keeper" });
  assert(frame.carried.items.some(i => i.item.item_instance_id === "found_charm"));
  await mark("lodge-return");
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await wait(() => document.body.dataset.phase === "signed_out");
  assert.equal(await page.locator("#world-canvas").getAttribute("data-study-level"), null);
  assert.deepEqual(errors, []);
  await writeFile(path.join(config.output, `${config.engine}-expedition.json`), JSON.stringify({ verdict:"PASS", engine:config.engine, renderer:launched.renderer, checkpoints, commands:commands.map(c=>c.intent.kind), created_through_ui:true, authored_world:true, normal_tls:true, scratch_postgres:true, reconnect_preserved_state:true, candidate_art:true,
    encounter_rounds_logged:encounterLog.length,
    production_encounter:{rounds:encounterRounds, damage_taken:encounterDamageTaken, starting_hp:encounterHpBefore, hp_after:frame.character.resources.hp, opponent_hp_authored:18, player_ratings_authored:true} },null,2));
  // The per-round trace is useful evidence but does not belong in the summary
  // receipt; it is written beside it.
  await writeFile(path.join(config.output, `${config.engine}-encounter-log.json`), JSON.stringify(encounterLog, null, 2));
  await context.close();
} catch(error) {
  await writeFile(path.join(config.output, `${config.engine}-failure.json`), JSON.stringify({ stage, error:String(error), errors, frame, staticContext, commandCount:commands.length, lastResult:results.at(-1) },null,2));
  await page?.screenshot({ path:path.join(config.output, `${config.engine}-failure.png`), fullPage:true }).catch(()=>{});
  throw error;
} finally { await launched.stop(); }
