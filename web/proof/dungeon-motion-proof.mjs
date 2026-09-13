import {collectGraphicsErrors} from './graphics-errors.mjs';
import assert from 'node:assert/strict';
import {readFile,writeFile} from 'node:fs/promises';
import {armPhases,savePhases} from './dungeon-phase-capture.mjs';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {worldCellPoint,waitForWorldPointing} from './world-pointing.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const session=await launchProofBrowser({name:config.engine,engine:PROOF_ENGINES[config.engine],trustedAuthority:config.authority});
let page,frame;const commands=[],results=[],updates=[],errors=[],captures=[];
const prefix=`${config.output}/${config.engine}-${config.scenario}`;
const {martialPlayback:{closingKick}}=JSON.parse(await readFile(new URL('../src/play/dungeon/receipt.json',import.meta.url),'utf8'));
const approach=config.approach??3;
try{
 const context=session.context||await session.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1600,height:1100});
 collectGraphicsErrors(page,errors);page.on('pageerror',e=>errors.push(String(e)));
 page.on('console',m=>{if(/THREE.*(binding|bone|track)|No target node found/i.test(m.text()))errors.push(m.text());});
 page.on('websocket',s=>{s.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m);});s.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;if(m.kind==='command_result')results.push(m);if(m.kind==='state_update')updates.push(m);});});
 const canvas=()=>page.locator('#world-canvas');
 const ready=()=>page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false',undefined,{timeout:45000});
 const motion=()=>canvas().evaluate((n,id)=>JSON.parse(n.dataset.worldMotions).find(m=>m.id===id),frame.observer_actor_id);
 const pose=clips=>page.waitForFunction(({id,clips})=>clips.includes(JSON.parse(document.querySelector('#world-canvas').dataset.worldMotions||'[]').find(m=>m.id===id)?.clip),{id:frame.observer_actor_id,clips},{timeout:15000});
 async function capture(name){
  const sample=await canvas().evaluate((n,id)=>({motion:JSON.parse(n.dataset.worldMotions).find(m=>m.id===id),png:n.toDataURL('image/png')}),frame.observer_actor_id);
  await writeFile(`${prefix}-${name}.png`,Buffer.from(sample.png.split(',')[1],'base64'));captures.push({name,motion:sample.motion,position:frame.observation_center});
 }
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
  const point=await canvas().evaluate((n,id)=>{const p=JSON.parse(n.dataset.worldActorPoints).find(p=>p.id===id),b=n.getBoundingClientRect();return {x:b.left+p.px*b.width,y:b.top+p.py*b.height};},target);
  await page.mouse.click(point.x,point.y,{button:'right'});const dialog=page.locator('.resident-dialog[open]');await dialog.waitFor();
  const before=commands.length,first=updates.length,position=structuredClone(frame.observation_center);
  const destination=structuredClone(frame.actors.find(a=>a.actor_id===target).position);
  const phases=mode==='jumpkick'?[
   {name:'jumpkick-takeoff',min:.05,max:.15},
   {name:'jumpkick-impact',min:closingKick.contactPhase,max:closingKick.contactPhase+.1},
   {name:'jumpkick-landing',min:closingKick.landingPhase,max:closingKick.landingPhase+.1},
   {name:'jumpkick-recovery',min:.8,max:.95},
   {name:'jumpkick-settled',settled:true},
  ]:[{name:'fight',min:.3,max:.5},{name:'fight-settled',settled:true}];
  await armPhases(page,frame.observer_actor_id,clips,phases);
  await dialog.locator(`button[data-action=${JSON.stringify(`character/${offer.id}`)}]`).click();
  await dialog.getByRole('button',{name:'Close',exact:true}).click();
  await accepted(before);assert.deepEqual(commands.at(-1).intent,offer.intent);
  const samples=await savePhases(page,prefix,phases);captures.push(...samples);
  const cues=updates.slice(first).flatMap(u=>u.events).filter(e=>e.kind==='feedback'&&e.cue.kind==='physical_combat'&&e.cue.mode===mode&&e.cue.source?.actor_id===frame.observer_actor_id);
  assert(cues.some(e=>['hit','missed','blocked'].includes(e.cue.outcome.kind)),'Animation requires confirmed physical feedback');
  if(mode==='jumpkick'){
   assert.deepEqual(frame.observation_center,destination,'The server must land the kick on the target tile');
   const movement=updates.slice(first).flatMap(u=>u.events).filter(e=>e.kind==='actor_moved'&&e.actor_id===frame.observer_actor_id);
   assert.equal(movement.length,approach,'This fixture must exercise the selected authoritative approach');
   let at=position;for(const step of movement){assert.deepEqual(step.from,at);assert.equal(step.navigation,'walk');at=step.to;}
   assert.deepEqual(at,destination,'Complete visible movement must explain the landing');
   const takeoff=samples.find(s=>s.name==='jumpkick-takeoff'),impact=samples.find(s=>s.name==='jumpkick-impact');
   assert(takeoff.motion.moving&&takeoff.motion.routeProgress>0&&takeoff.motion.routeProgress<1);
   assert.equal(takeoff.motion.route.length,movement.length+1);
   for(const [i,point]of takeoff.motion.route.entries()){
    const authoritative=i===0?position.position:movement[i-1].to.position;
    assert(Math.abs(point.x-authoritative.x)<=.260001&&Math.abs(point.y-authoritative.y)<=.260001);
   }
   const route=takeoff.motion.route,lengths=route.slice(1).map((p,i)=>Math.hypot(p.x-route[i].x,p.y-route[i].y));
   let distance=lengths.reduce((a,b)=>a+b,0)*takeoff.motion.routeProgress,expected;
   for(let i=0;i<lengths.length;i++){
    if(distance<=lengths[i]||i===lengths.length-1){const t=distance/lengths[i];expected={x:route[i].x+(route[i+1].x-route[i].x)*t,y:route[i].y+(route[i+1].y-route[i].y)*t};break;}distance-=lengths[i];
   }
   assert(Math.hypot(takeoff.anchor.x-expected.x,takeoff.anchor.y-expected.y)<1e-6);
   assert.equal(impact.motion.routeProgress,1);assert.equal(impact.motion.moving,false);
   for(const sample of samples.filter(s=>s!==takeoff)){
    assert(Math.hypot(sample.anchor.x-impact.anchor.x,sample.anchor.y-impact.anchor.y)<1e-6,'Impact, landing and recovery stay at the accepted endpoint');
   }
  }else assert.deepEqual(frame.observation_center,position,'A punch must not move the attacker');
  await pose(['guard']);assert.equal((await motion()).moving,false);
 }
 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();await ready();await waitForWorldPointing(page);
 assert.equal(frame.character.identity.base_class_id,'martial_artist');assert.equal(frame.character.identity.sex_or_gender_display,config.sex);
 assert.equal((await motion()).body,config.sex);assert.equal((await motion()).clip,'guard');assert.equal((await motion()).moving,false);await capture('guard');
 await move(22,9,'closed-door',true);await move(21,9,'walk');await move(24,9,'open-door-sprint',true);
 if(approach!==1)await move(25-approach,9,'kick-origin',approach===3);else await capture('kick-origin');
 await attack('jumpkick',['flying_kick']);await attack('fight',['jab_left','jab_right','uppercut_right','hook_left'],'motion_fist');
 await page.getByRole('button',{name:'Reconnect',exact:true}).click();await ready();await pose(['guard']);await capture('reconnected');assert.deepEqual(errors,[]);
 await page.getByRole('button',{name:'Sign out',exact:true}).click();await page.waitForFunction(()=>document.body.dataset.phase==='signed_out');
 assert.equal(await canvas().getAttribute('data-world-motions'),null);
 for(const file of ['martial-male.glb','martial-female.glb'])for(const failure of ['missing','digest']){
  const pattern=`**/feel-assets/${file}`;await page.route(pattern,r=>r.fulfill({status:failure==='missing'?404:200,body:'not the bound model'}));
  await page.reload();await page.waitForFunction(()=>document.body.dataset.playReady==='failed');assert(await page.locator('#login').isDisabled());await page.unroute(pattern);
 }
 await writeFile(`${prefix}.json`,JSON.stringify({verdict:'PASS',renderer:session.renderer,sex:config.sex,approach,captures,commands:commands.map(c=>c.intent),confirmedFeedback:true,reconnectGuard:true,bothRequiredAssetsRefused:true,errors},null,2));console.log(`PASS ${config.engine}/${config.scenario}`);
}catch(error){await page?.screenshot({path:`${prefix}-failure.png`}).catch(()=>{});await writeFile(`${prefix}-failure.json`,JSON.stringify({error:String(error),errors,frame,commands,results},null,2));throw error;}finally{await session.stop();}
