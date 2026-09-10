import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {worldCellPoint,waitForWorldPointing} from './world-pointing.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const session=await launchProofBrowser({name:config.engine,engine:PROOF_ENGINES[config.engine],trustedAuthority:config.authority});
let page,frame;const commands=[],results=[],updates=[],errors=[],captures=[];
const prefix=`${config.output}/${config.engine}-${config.scenario}`;
try{
 const context=session.context||await session.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1600,height:1100});
 page.on('pageerror',e=>errors.push(String(e)));
 page.on('console',m=>{if(/THREE.*(binding|bone|track)|No target node found/i.test(m.text()))errors.push(m.text());});
 page.on('websocket',s=>{s.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m);});s.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;if(m.kind==='command_result')results.push(m);if(m.kind==='state_update')updates.push(m);});});
 const canvas=()=>page.locator('#world-canvas');
 const ready=()=>page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false',undefined,{timeout:45000});
 const motion=()=>canvas().evaluate((n,id)=>JSON.parse(n.dataset.dungeonMotions).find(m=>m.id===id),frame.observer_actor_id);
 const pose=clips=>page.waitForFunction(({id,clips})=>clips.includes(JSON.parse(document.querySelector('#world-canvas').dataset.dungeonMotions||'[]').find(m=>m.id===id)?.clip),{id:frame.observer_actor_id,clips},{timeout:15000});
 async function capture(name){await canvas().screenshot({path:`${prefix}-${name}.png`});captures.push({name,motion:await motion(),position:frame.observation_center});}
 async function accepted(before){await ready();assert.equal(commands.length,before+1);const result=results.find(r=>r.command_id===commands.at(-1).command_id);assert.equal(result?.disposition.kind,'accepted');}
 async function move(x,y,name,door=false){
  await ready();const before=commands.length,first=updates.length,p=await worldCellPoint(page,{x,y});await page.mouse.click(p.x,p.y);
  await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.walkState==='draft');await page.mouse.click(p.x,p.y);
  await pose(['walk']);assert.equal((await motion()).moving,true);await page.waitForTimeout(250);await capture(name);await accepted(before);await pose(['guard']);
  assert.deepEqual(frame.observation_center.position,{x,y});
  if(door)assert(updates.slice(first).flatMap(u=>u.events).some(e=>e.kind==='actor_moved'&&e.actor_id===frame.observer_actor_id&&e.navigation==='door'),'Real local-door step must reach motion playback');
 }
 async function attack(mode,clips,target='motion_target'){
  await ready();const offer=frame.action_options.find(a=>a.enabled&&a.intent?.kind==='physical_attack'&&a.intent.mode===mode&&a.intent.target_actor_id===target);
  assert(offer,`No authoritative ${mode} offer: ${JSON.stringify(frame.action_options)}`);
  const point=await canvas().evaluate((n,id)=>{const p=JSON.parse(n.dataset.dungeonActorPoints).find(p=>p.id===id),b=n.getBoundingClientRect();return {x:b.left+p.px*b.width,y:b.top+p.py*b.height};},target);
  await page.mouse.click(point.x,point.y,{button:'right'});const dialog=page.locator('.resident-dialog[open]');await dialog.waitFor();
  const before=commands.length,first=updates.length,position=structuredClone(frame.observation_center);
  await dialog.locator(`button[data-action=${JSON.stringify(`character/${offer.id}`)}]`).click();
  await pose(clips);await dialog.getByRole('button',{name:'Close',exact:true}).click();await page.waitForTimeout(400);await capture(mode);
  await accepted(before);assert.deepEqual(commands.at(-1).intent,offer.intent);
  const cues=updates.slice(first).flatMap(u=>u.events).filter(e=>e.kind==='feedback'&&e.cue.kind==='physical_combat'&&e.cue.mode===mode&&e.cue.source?.actor_id===frame.observer_actor_id);
  assert(cues.some(e=>['hit','missed','blocked'].includes(e.cue.outcome.kind)),'Animation requires confirmed physical feedback');
  assert.deepEqual(frame.observation_center,position,'Animation must not fabricate movement');await pose(['guard']);
 }
 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();await ready();await waitForWorldPointing(page);
 assert.equal(frame.character.identity.base_class_id,'martial_artist');assert.equal(frame.character.identity.sex_or_gender_display,config.sex);
 assert.deepEqual(await motion(),{id:frame.observer_actor_id,body:config.sex,clip:'guard',moving:false});await capture('guard');
 await move(22,9,'closed-door',true);await move(21,9,'walk');await move(24,9,'open-door-sprint',true);await attack('jumpkick',['flying_kick']);await move(25,8,'engage');await attack('fight',['jab_left','jab_right','uppercut_right','hook_left'],'motion_fist');
 await page.getByRole('button',{name:'Reconnect',exact:true}).click();await ready();await pose(['guard']);await capture('reconnected');assert.deepEqual(errors,[]);
 await page.getByRole('button',{name:'Sign out',exact:true}).click();await page.waitForFunction(()=>document.body.dataset.phase==='signed_out');
 assert.equal(await canvas().getAttribute('data-dungeon-motions'),null);
 for(const file of ['martial-male.glb','martial-female.glb'])for(const failure of ['missing','digest']){
  const pattern=`**/feel-assets/${file}`;await page.route(pattern,r=>r.fulfill({status:failure==='missing'?404:200,body:'not the bound model'}));
  await page.reload();await page.waitForFunction(()=>document.body.dataset.playReady==='failed');assert(await page.locator('#login').isDisabled());await page.unroute(pattern);
 }
 await writeFile(`${prefix}.json`,JSON.stringify({verdict:'PASS',renderer:session.renderer,sex:config.sex,captures,commands:commands.map(c=>c.intent),confirmedFeedback:true,reconnectGuard:true,bothRequiredAssetsRefused:true,errors},null,2));console.log(`PASS ${config.engine}/${config.scenario}`);
}catch(error){await page?.screenshot({path:`${prefix}-failure.png`}).catch(()=>{});await writeFile(`${prefix}-failure.json`,JSON.stringify({error:String(error),errors,frame,commands,results},null,2));throw error;}finally{await session.stop();}
