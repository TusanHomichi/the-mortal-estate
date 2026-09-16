import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {setTimeout as delay} from 'node:timers/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {collectGraphicsErrors} from './graphics-errors.mjs';
import {checkpointLedger,restartServingProcess} from './proof-handshake.mjs';
import {worldCellPoint} from './world-pointing.mjs';

let input='';for await(const chunk of process.stdin)input+=chunk;
const config=JSON.parse(input);input='';
const engine=PROOF_ENGINES[config.engine];assert(engine);
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
const name=`${config.engine}-${config.alignment}`,sent=[],received=[],errors=[],captures=[];
let page,frame,stage='startup';
async function eventually(predicate,timeout=45000){
  const end=Date.now()+timeout;
  while(!predicate()){if(Date.now()>end)throw Error(`Timed out: ${stage}`);await delay(25);}
}
try {
  const context=launched.context||await launched.browser.newContext();
  page=await context.newPage();await page.setViewportSize({width:1280,height:900});await page.bringToFront();
  collectGraphicsErrors(page,errors);page.on('pageerror',error=>errors.push(error.message));
  page.on('websocket',socket=>{
    socket.on('framesent',event=>{const value=JSON.parse(String(event.payload));if(value.kind!=='client_hello')sent.push(value);});
    socket.on('framereceived',event=>{const value=JSON.parse(String(event.payload));received.push(value);if(value.frame)frame=value.frame;});
  });
  const wait=(fn,arg,timeout=45000)=>page.waitForFunction(fn,arg,{timeout,polling:30});
  const canvas=page.locator('#world-canvas');
  const ready=(timeout=45000)=>wait(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false',null,timeout);
  const commands=()=>sent.filter(value=>value.kind==='command');
  // Capture the playable viewport, including its speech and death controls.
  // Element capture adds a scroll/stability wait to the continuously drawn canvas.
  const mark=async label=>{await page.screenshot({path:`${config.output}/${name}-${label}.png`});captures.push(label);};
  await page.goto(config.origin+'/');await wait(()=>document.body.dataset.playReady==='true');
  await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
  await page.locator('#login').click();await wait(()=>document.body.dataset.phase==='selecting');
  await page.locator('#enter').click();await ready();
  const actor=frame.observer_actor_id,character=frame.social.character_id;
  assert.equal(frame.actors.find(row=>row.actor_id===actor).life_state,'alive');
  assert.equal(await canvas.getAttribute('data-presentation'),'world-3d');
  // Everything the character carries at creation, and how much of it is held
  // rather than worn: a corpse keeps worn equipment for its owner while a
  // scavenging opponent may legitimately take what the dying character dropped.
  const items=structuredClone(frame.carried.items);
  const goldBefore=frame.carried.gold.sack;
  // Ordinary death is production combat: the character walks onto the shipped
  // opponent and then stands there. Nothing here grants, removes or assigns HP,
  // and the scavenger keeps its authored attack and its scavenging profile.
  // Each step is an ordinary movement command that also lets the monster act.
  stage='enter combat';
  const self=()=>frame.actors.find(row=>row.actor_id===actor);
  const hpBefore=self().hp;
  // The authoritative item and coin ledger, read from the stored checkpoint by
  // the runner while the character is still alive and about to join the
  // encounter. Everything the world owns is accounted for from here on: an
  // instance may be dropped, retained in the corpse or taken by the scavenging
  // opponent, but it may not stop existing, duplicate, change identity or
  // quantity, and no coin may be minted or burned.
  const ledgerBefore=await checkpointLedger(config,'capture','before-death',eventually);
  assert.equal(ledgerBefore.verdict,'captured',JSON.stringify(ledgerBefore));
  await canvas.focus();
  // One real East movement puts the character on the opponent's square; every
  // later step is the ordinary Wait action. The opponent acts on its own
  // deadline, so the loop advances by observed readiness rather than by a
  // guessed number of rounds, and only the bound is a failure.
  // Keyboard bindings are the client's own input path: ArrowRight walks one
  // step and Space waits. Neither depends on whether the diagnostic control
  // column happens to be laid out at this viewport.
  // The observed frame is the authority; the DOM attribute is only a wait target
  // and is not reachable from this half of the proof.
  const dead=()=>self()?.life_state==='ghost';
  const step=async(key)=>{
    // Death can land between the readiness wait and the input, so readiness
    // yields to an observed ghost state instead of failing the case.
    await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.canAct==='true'||
      document.querySelector('#world-canvas').dataset.lifeState==='ghost',null,{polling:30,timeout:90000});
    if(dead())return;
    const before=commands().length;
    await page.keyboard.press(key);
    await eventually(()=>commands().length>before||dead(),60000);
    await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.pending==='false',null,{polling:30,timeout:90000});};
  let damageTaken=0;
  await step('ArrowRight');
  for(let round=0;round<200;round+=1){
    damageTaken=hpBefore-self().hp;
    if(self().life_state==='ghost')break;
    await step('Space');
  }
  await wait(()=>document.querySelector('#world-canvas').dataset.lifeState==='ghost',null,60000);
  assert(damageTaken>0,'production combat must be what removed the character, not a fixture');
  assert(hpBefore>0,'the character started the encounter with its authored starting health');
  assert(received.some(value=>value.events?.some(event=>event.kind==='feedback'&&event.cue.kind==='defeat')),'real defeat feedback must reach the browser');
  const corpse=structuredClone(frame.observation_center),deadline=frame.ready_at;
  assert.deepEqual(corpse.position,{x:24,y:9});
  assert(BigInt(deadline)-BigInt(frame.logical_time)>45000n,'observe death before eligibility');
  assert(frame.tiles.length>1&&frame.tiles.some(tile=>tile.position.x!==corpse.position.x||tile.position.y!==corpse.position.y));
  assert(frame.actors.some(row=>row.actor_id!==actor));
  assert(frame.corpses.some(row=>JSON.stringify(row.location)===JSON.stringify(corpse)));
  const motion=()=>canvas.evaluate((node,id)=>JSON.parse(node.dataset.worldMotions).find(row=>row.id===id),actor);
  await wait(id=>JSON.parse(document.querySelector('#world-canvas').dataset.worldMotions).some(row=>row.id===id&&row.ghost),actor);
  assert.equal((await motion()).body,config.body);assert.equal((await motion()).moving,false);
  assert(await page.getByRole('region',{name:'Death and return',exact:true}).isVisible());
  assert(await page.getByRole('button',{name:'Request resurrection',exact:true}).isDisabled());
  assert(await page.getByRole('form',{name:'Local speech',exact:true}).isVisible());
  await mark('ghost');
  stage='ghost speech';const sequence=await canvas.getAttribute('data-sequence');
  await page.locator('input[name="say"]').fill('I am still here.');await page.getByRole('button',{name:'Say',exact:true}).click();
  // Browser input completion does not order Playwright's asynchronous socket
  // notifications. Observe the actual outgoing frame before checking its result.
  await eventually(()=>sent.some(value=>value.kind==='social_message'));
  const speech=sent.find(value=>value.kind==='social_message');assert(speech);
  await eventually(()=>received.some(value=>value.kind==='message_result'&&value.message_id===speech.message_id&&value.disposition==='accepted'));
  await wait(()=>document.querySelector('[role="log"]').textContent.includes('I am still here.'));
  assert.equal(await canvas.getAttribute('data-sequence'),sequence);
  stage='reconnect before eligibility';await page.locator('#reconnect').click();
  await wait(()=>document.body.dataset.phase==='playing');
  assert.equal(frame.observer_actor_id,actor);assert.equal(frame.social.character_id,character);
  assert.equal(frame.ready_at,deadline);assert.deepEqual(frame.observation_center,corpse);assert.equal(frame.can_act,false);
  assert.equal(await page.locator('[role="log"]').textContent(),'');
  // Restart the serving process under the ghost and prove the death state is
  // durable. The runner owns the process, so the two halves coordinate through
  // files; no gameplay state crosses the boundary, and the page is reloaded
  // because the restarted process listens on a new port.
  stage='restart while dead';
  const restart=await restartServingProcess(config,
    {engine:config.engine,alignment:config.alignment,actor_id:actor,character_id:character,
     ready_at:deadline,location:corpse,life_state:'ghost'},eventually);
  assert.equal(restart.payload_comparison,'parsed_json_minus_named_volatile_fields',
    'the restart receipt must name how it compared the checkpoint');
  assert.equal(restart.durable_payload_unchanged,true,'the durable checkpoint payload changed across the restart');
  assert.deepEqual(restart.excluded_volatile_fields,['character_presence'],'only the live presence mark may move');
  // The raw stored digest is reported as an observation, not asserted: a restart
  // drops the old session, so the presence mark inside the checkpoint may move
  // even when every durable gameplay field is identical.
  assert.equal(typeof restart.checkpoint_bytes_identical,'boolean','the raw digest comparison must be reported');
  assert.equal(restart.physical_action_refused_while_dead,true,'a restarted ghost was allowed to act');
  const restartedOrigin=restart.origin||config.origin;
  config.origin=restartedOrigin;
  await page.goto(restartedOrigin+'/');await wait(()=>document.body.dataset.playReady==='true');
  await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
  await page.locator('#login').click();await wait(()=>document.body.dataset.phase==='selecting');
  await page.locator('#enter').click();await ready(75000);
  assert.equal(frame.observer_actor_id,actor);assert.equal(frame.social.character_id,character);
  assert.equal(frame.actors.find(row=>row.actor_id===actor).life_state,'ghost','the restart resurrected the character');
  assert.deepEqual(frame.observation_center,corpse,'the restart moved the ghost');
  assert.equal(frame.ready_at,deadline,'the restart reset the return eligibility deadline');
  assert.equal(await canvas.getAttribute('data-life-state'),'ghost');
  assert(await page.getByRole('form',{name:'Local speech',exact:true}).isVisible(),'the restarted ghost lost its speech');
  // The threshold may or may not have elapsed by the time the restarted process
  // has been re-entered. Both answers are correct; what is not correct is
  // offering a return before the character's own deadline, so the state is
  // recorded and the existing eligibility step below still owns the wait.
  const request=page.getByRole('button',{name:'Request resurrection',exact:true});
  const restartedOffer={enabled:!(await request.isDisabled())};
  assert(!restartedOffer.enabled||BigInt(frame.logical_time)>=BigInt(deadline),
    'the restarted server offered a return before the standard threshold');
  await mark('restarted-ghost');
  const count=commands().length,previewCount=sent.filter(value=>value.kind==='path_preview').length;
  for(const button of await page.locator('[data-action]').all())assert(await button.isDisabled());
  const point=await worldCellPoint(page,{x:23,y:9});await page.mouse.click(point.x,point.y);
  await page.keyboard.press('ArrowLeft');
  assert.equal(commands().length,count);assert.equal(sent.filter(value=>value.kind==='path_preview').length,previewCount);
  stage='authoritative eligibility';await ready(75000);
  assert.equal(frame.ready_at,deadline);assert(BigInt(frame.logical_time)>=BigInt(deadline));
  assert.equal(frame.actors.find(row=>row.actor_id===actor).life_state,'ghost');assert.deepEqual(frame.observation_center,corpse);
  assert.equal((await motion()).moving,false);
  for(const button of await page.locator('[data-action]').all())assert(await button.isDisabled());
  assert.deepEqual(frame.action_options.filter(row=>row.enabled).map(row=>row.intent),[{kind:'request_resurrection'}]);
  await page.mouse.click(point.x,point.y);assert.equal(commands().length,count);
  stage='return request';const returnStart=received.length;
  await page.getByRole('button',{name:'Request resurrection',exact:true}).click();
  await wait(()=>document.querySelector('#world-canvas').dataset.lifeState==='alive');
  assert.equal(commands().length,count+1);assert.deepEqual(commands().at(-1).intent,{kind:'request_resurrection'});
  const returnFrames=received.slice(returnStart).filter(value=>value.frame?.actors.some(row=>row.actor_id===actor&&row.life_state==='alive'));
  assert(returnFrames.length);const returning=returnFrames[0].frame;
  assert.equal(BigInt(returning.ready_at)-BigInt(returning.logical_time),3000n);assert.equal(returning.can_act,false);
  assert.equal(returning.observer_actor_id,actor);assert.equal(returning.social.character_id,character);
  assert.deepEqual(returning.observation_center,config.destination);
  // Conservation, not a happy ending. Production scavenging stays enabled, and
  // a scavenging opponent legitimately strips the corpse: the shipped profile
  // searches corpses, collects ground items and gold, and equips what it finds.
  // Asserting that the character's creation loadout returns would therefore be
  // the wrong oracle. What is checked is what the return itself can create:
  // nothing. No duplicated item, no created gold, and the same character.
  const returnedIds=new Set(returning.carried.items.map(row=>row.item.item_instance_id));
  const unobserved=items.map(row=>row.item.item_instance_id).filter(id=>!returnedIds.has(id));
  // The counted oracle above is diagnostic only. Conservation is decided by the
  // runner's audit of the authoritative ledger, taken against the capture made
  // at the death boundary; a count comparison would accept an emptied inventory,
  // a same-count substitution or a duplicated instance.
  const ledgerAfter=await checkpointLedger(config,'audit','before-death',eventually);
  assert.equal(ledgerAfter.verdict,'audited',JSON.stringify(ledgerAfter.defects));
  assert.deepEqual(ledgerAfter.defects,[],'the death boundary must conserve every item and coin');
  assert.equal(ledgerAfter.after.item_count,ledgerAfter.before.item_count,'every instance still exists');
  assert.equal(ledgerAfter.after.gold_total,ledgerAfter.before.gold_total,'every coin still exists');
  assert.deepEqual(ledgerAfter.after.unowned,[],'every instance has exactly one owner');
  assert(received.slice(returnStart).some(value=>value.kind==='command_result'&&value.command_id===commands().at(-1).command_id&&value.disposition.kind==='accepted'));
  await ready();assert.equal((await motion()).ghost,false);await mark('returned');
  stage='living movement';await canvas.focus();await page.keyboard.press('ArrowUp');
  await eventually(()=>commands().length===count+2&&JSON.stringify(frame.observation_center)!==JSON.stringify(config.destination));await ready();
  assert.notDeepEqual(frame.observation_center,config.destination);
  await page.locator('#logout').click();await wait(()=>document.body.dataset.phase==='signed_out');
  assert.equal(await canvas.getAttribute('data-life-state'),'');assert.deepEqual(errors,[]);
  await writeFile(`${config.output}/${name}.json`,JSON.stringify({verdict:'PASS',engine:config.engine,renderer:launched.renderer,
    alignment:config.alignment,body:config.body,captures,death_deadline:deadline,corpse,return_destination:config.destination,
    reconnect_preserves_death:true,ghost_speech:true,stationary_ghost:true,return_cooldown_ms:3000,
    restart_while_dead:restart,restart_offer:restartedOffer,
    production_combat_damage:damageTaken,starting_hp:hpBefore,production_content:true,
    scavenging_enabled:true,items_at_creation:items,items_after_return:returning.carried.items,
    gold_at_creation:goldBefore,return_carried_gold:returning.carried.gold.sack,
    items_at_creation_count:items.length,items_after_return_count:returning.carried.items.length,
    items_not_visible_from_the_return_destination:unobserved,
    ledger_before:ledgerBefore,ledger_after:ledgerAfter,
    commands:commands().map(command=>command.intent),errors},null,2)+'\n');
}catch(error){
  await page?.screenshot({path:`${config.output}/${name}-failure.png`}).catch(()=>{});
  await writeFile(`${config.output}/${name}-failure.json`,JSON.stringify({verdict:'FAIL',stage,error:String(error),frame,errors,
    feedback:received.flatMap(value=>value.events??[]).filter(event=>event.kind==='feedback').slice(-30),
    commands:sent.filter(value=>value.kind==='command').map(value=>value.intent)},null,2)+'\n');throw error;
}finally{await launched.stop();}
