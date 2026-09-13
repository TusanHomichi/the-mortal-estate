import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {setTimeout as delay} from 'node:timers/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {collectGraphicsErrors} from './graphics-errors.mjs';

let input='';for await(const chunk of process.stdin)input+=chunk;
const config=JSON.parse(input);input='';
const engine=PROOF_ENGINES[config.engine];assert(engine);
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
const name=`${config.engine}-${config.alignment}`,sent=[],received=[],errors=[];
let page,frame,stage='startup';
async function eventually(predicate){
  const end=Date.now()+45000;
  while(!predicate()){if(Date.now()>end)throw Error(`Timed out: ${stage}`);await delay(25);}
}
try {
  const context=launched.context||await launched.browser.newContext();
  page=await context.newPage();await page.setViewportSize({width:1280,height:900});await page.bringToFront();
  collectGraphicsErrors(page,errors);page.on('pageerror',error=>errors.push(error.message));
  page.on('websocket',socket=>{
    socket.on('framesent',event=>sent.push(JSON.parse(String(event.payload))));
    socket.on('framereceived',event=>{const value=JSON.parse(String(event.payload));received.push(value);if(value.frame)frame=value.frame;});
  });
  const wait=(fn,arg)=>page.waitForFunction(fn,arg,{timeout:45000,polling:30});
  const canvas=page.locator('#world-canvas');
  const ready=()=>wait(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false');
  await page.goto(config.origin+'/');await wait(()=>document.body.dataset.playReady==='true');
  await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
  await page.locator('#login').click();await wait(()=>document.body.dataset.phase==='selecting');
  await page.locator('#enter').click();await ready();
  const actor=frame.observer_actor_id,character=frame.social.character_id,deathSite=structuredClone(frame.observation_center);
  assert.equal(await canvas.getAttribute('data-presentation'),'world-3d');
  assert(frame.carried.items.length>0);assert.equal(frame.character.resources.hp,frame.character.resources.max_hp);
  stage='enter fire range';const start=received.length;
  await canvas.focus();await page.keyboard.press('ArrowRight');
  await eventually(()=>received.slice(start).some(value=>value.frame&&JSON.stringify(value.frame.observation_center)===JSON.stringify(config.destination)));
  const frames=received.slice(start).filter(value=>value.frame);
  const returning=frames.find(value=>JSON.stringify(value.frame.observation_center)===JSON.stringify(config.destination))?.frame;assert(returning);
  assert(frames.every(value=>value.frame.actors.find(row=>row.actor_id===actor)?.life_state==='alive'),'no published ghost or awaiting interval');
  assert(received.slice(start).some(value=>value.events?.some(event=>event.kind==='feedback'&&event.cue.kind==='defeat')));
  assert.equal(returning.observer_actor_id,actor);assert.equal(returning.social.character_id,character);
  assert.equal(returning.character.resources.hp,returning.character.resources.max_hp-1);
  assert.equal(returning.carried.items.length,0);assert(Object.values(returning.carried.gold).every(value=>BigInt(value)===0n));
  const remaining=BigInt(returning.ready_at)-BigInt(returning.logical_time);
  assert(remaining>0n&&remaining<=3000n,'publication may follow the scheduled death instant');assert.equal(returning.can_act,false);
  assert(!returning.action_options.some(action=>action.intent?.kind==='request_resurrection'));
  stage='reconnect after return';const reconnectStart=received.length;await page.locator('#reconnect').click();
  await eventually(()=>received.slice(reconnectStart).some(value=>value.frame?.observer_actor_id===actor));
  await ready();assert.deepEqual(frame.observation_center,config.destination);assert.equal(frame.observer_actor_id,actor);assert.equal(frame.social.character_id,character);
  assert.equal(frame.ready_at,returning.ready_at);
  await wait(id=>JSON.parse(document.querySelector('#world-canvas').dataset.worldMotions).some(row=>row.id===id&&!row.ghost),actor);
  const motion=await canvas.evaluate((node,id)=>JSON.parse(node.dataset.worldMotions).find(row=>row.id===id),actor);assert.equal(motion.body,config.body);
  await page.screenshot({path:`${config.output}/${name}-returned.png`});
  stage='living movement';await canvas.focus();await page.keyboard.press('ArrowUp');
  await eventually(()=>JSON.stringify(frame.observation_center)!==JSON.stringify(config.destination));await ready();
  await page.locator('#logout').click();await wait(()=>document.body.dataset.phase==='signed_out');assert.deepEqual(errors,[]);
  const commands=sent.filter(value=>value.kind==='command').map(value=>value.intent);
  assert.equal(commands.length,2);assert.equal(commands[0].kind,'move_path');assert.equal(commands[1].kind,'move_path');
  await writeFile(`${config.output}/${name}.json`,JSON.stringify({verdict:'PASS',engine:config.engine,renderer:launched.renderer,
    alignment:config.alignment,body:config.body,death_site:deathSite,return_destination:config.destination,
    immediate_return:true,no_ghost_interval:true,empty_inventory:true,reconnect_preserves_return:true,
    return_cooldown_ms:3000,observed_remaining_ms:Number(remaining),commands,errors},null,2)+'\n');
}catch(error){
  await page?.screenshot({path:`${config.output}/${name}-failure.png`}).catch(()=>{});
  await writeFile(`${config.output}/${name}-failure.json`,JSON.stringify({verdict:'FAIL',stage,error:String(error),frame,errors,
    messages:received.slice(-3),commands:sent.filter(value=>value.kind==='command').map(value=>value.intent)},null,2)+'\n');throw error;
}finally{await launched.stop();}
