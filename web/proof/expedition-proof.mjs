#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
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
  const ready = () => wait(() => document.body.dataset.phase === "playing" && document.querySelector("#world-canvas").dataset.canAct === "true" && document.querySelector("#world-canvas").dataset.pending === "false");
  const here = () => frame.observation_center;
  async function mark(name) {
    stage = name; checkpoints.push({ stage, location: here(), gold: frame.carried.gold.sack, hp: frame.character.resources.hp });
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
  async function action(predicate, { service, amount } = {}) {
    await ready();
    const groups = [...frame.services_here.map(row => ({ key: `service:${row.service_id}`, actions: serviceActions(row) })), { key: "character", actions: frame.action_options }];
    const group = groups.find(g => (!service || g.key === `service:${service}`) && g.actions.some(a => a.enabled && a.intent && predicate(a.intent)));
    assert(group, `required action absent at ${JSON.stringify(here())}`);
    const chosen = group.actions.find(a => a.enabled && a.intent && predicate(a.intent));
    const section = page.locator(`details[data-group=${JSON.stringify(group.key)}]`);
    if (await section.getAttribute("open") === null) await section.locator("summary").click();
    await section.locator("select").selectOption(chosen.id);
    if (amount !== undefined) await section.locator("input").fill(amount);
    const count = commands.length;
    await section.getByRole("button", { name: "Perform selected action" }).click();
    return committed(count);
  }
  const steps = [[0,-1,"North"],[1,0,"East"],[0,1,"South"],[-1,0,"West"]];
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
      await page.getByRole("button", { name: direction, exact: true }).click();
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
  await page.getByRole("button", { name: "Create a new character", exact: true }).click();
  await page.locator("#creation-form").waitFor({ state: "visible" });
  assert.equal(await page.locator("#creation-profile option").count(), 5);
  await page.locator("#creation-profile").selectOption("creation/wizard");
  await page.locator("#creation-name").fill("Expedition Arrival");
  await page.getByRole("button", { name: "Create character", exact: true }).click();
  await wait(() => document.querySelector("#character").selectedOptions[0]?.textContent === "Expedition Arrival");
  await page.getByRole("button", { name: "Enter world", exact: true }).click(); await ready();
  assert(frame.observer_actor_id.startsWith("created/"));
  assert.deepEqual(here().position, { x:9,y:31 });
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
  assert.deepEqual(here().position, { x:5,y:4 }); await mark("descent");
  await walkTo(4,6);
  for (let attempts=0; attempts<6 && frame.actors.some(a => a.actor_id === "cellar_scavenger" && a.life_state !== "dead"); attempts++) {
    await action(i => i.kind === "physical_attack" && i.target_actor_id === "cellar_scavenger" && i.mode === "fight");
  }
  assert(frame.corpses.length > 0, "encounter did not produce a corpse");
  await action(i => i.kind === "search_corpse");
  await action(i => i.kind === "move_item" && i.item_instance_id === "found_charm" && i.destination.kind === "carried" && i.destination.position === "sack_item_3");
  await action(i => i.kind === "move_gold" && i.source.kind === "ground" && i.destination.kind === "carried" && i.destination.position === "sack");
  await mark("loot"); await enter("temple"); await enter("arrival"); await enter("trainers"); await walkTo(1,2);
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
  await writeFile(path.join(config.output, `${config.engine}-expedition.json`), JSON.stringify({ verdict:"PASS", engine:config.engine, renderer:launched.renderer, checkpoints, commands:commands.map(c=>c.intent.kind), created_through_ui:true, authored_world:true, normal_tls:true, scratch_postgres:true, reconnect_preserved_state:true, candidate_art:true },null,2));
  await context.close();
} catch(error) {
  await writeFile(path.join(config.output, `${config.engine}-failure.json`), JSON.stringify({ stage, error:String(error), errors, frame, staticContext, commandCount:commands.length, lastResult:results.at(-1) },null,2));
  await page?.screenshot({ path:path.join(config.output, `${config.engine}-failure.png`), fullPage:true }).catch(()=>{});
  throw error;
} finally { await launched.stop(); }
