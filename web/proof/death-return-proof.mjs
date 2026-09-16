import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {setTimeout as delay} from 'node:timers/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {collectGraphicsErrors} from './graphics-errors.mjs';
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
  const items=structuredClone(frame.carried.items),retained=items.filter(row=>row.position.startsWith('sack_')||row.position.startsWith('belt_'));
  // Ordinary death is production combat: the character walks onto the shipped
  // opponent and then stands there. Nothing here grants, removes or assigns HP,
  // and the scavenger keeps its authored attack and its scavenging profile.
  // Each step is an ordinary movement command that also lets the monster act.
  stage='enter combat';
  const hpBefore=frame.actors.find(row=>row.actor_id===actor).hp;
  await canvas.focus();
  let damageTaken=0;
  for(let step=0;step<120;step+=1){
    const self=frame.actors.find(row=>row.actor_id===actor);
    damageTaken=hpBefore-self.hp;
    if(self.life_state==='ghost')break;
    await page.keyboard.press('ArrowRight');
    await eventually(()=>frame.actors.find(row=>row.actor_id===actor).hp<hpBefore||
      frame.actors.find(row=>row.actor_id===actor).life_state==='ghost',30000);
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
  for(const item of retained)assert(returning.carried.items.some(row=>row.item.item_instance_id===item.item.item_instance_id));
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
    production_combat_damage:damageTaken,starting_hp:hpBefore,production_content:true,
    commands:commands().map(command=>command.intent),errors},null,2)+'\n');
}catch(error){
  await page?.screenshot({path:`${config.output}/${name}-failure.png`}).catch(()=>{});
  await writeFile(`${config.output}/${name}-failure.json`,JSON.stringify({verdict:'FAIL',stage,error:String(error),frame,errors,
    feedback:received.flatMap(value=>value.events??[]).filter(event=>event.kind==='feedback').slice(-30),
    commands:sent.filter(value=>value.kind==='command').map(value=>value.intent)},null,2)+'\n');throw error;
}finally{await launched.stop();}
