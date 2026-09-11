// The observed character is the DEFENDER. This is the inverse of the motion proof:
// no command is sent to attack. An authored hold-ground monster shares the tile
// and the server's own automatic attack path produces the swing.
//
// What a PASS establishes is stated in the report's `guarantee` field and in the
// owning slice record. The client's motion diagnostics expose only
// `{id, body, clip, moving}`, so a rendered clip cannot be bound to the individual
// state update that caused it. This script therefore reports bracketed
// observations of authoritative swings and of block playback, rather than claiming
// atomic frame-level attribution.
import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {waitForWorldPointing} from './world-pointing.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const session=await launchProofBrowser({name:config.engine,engine:PROOF_ENGINES[config.engine],trustedAuthority:config.authority});
let page,frame;const commands=[],updates=[],errors=[],captures=[];
const prefix=`${config.output}/${config.engine}-${config.scenario}`;
// A blocked incoming `fight` is presented as this clip; the wire outcome names no
// block source, so the clip is chosen by attack mode alone (dungeon/motion.ts).
const BLOCK_CLIP='block_high';
// The motion owner separates real swings from no_sight/not_ready, which mean the
// actor never attacked. Only these outcomes count as an attack for this scenario.
const SWING_OUTCOMES=new Set(['hit','missed','blocked']);
// Any posture a defender might legitimately hold while an unblocked swing lands.
const NOT_BLOCKING=['guard','block_high','block_side','block_cover','block_lean'];
const negative=config.occupied===true;
try{
 const context=session.context||await session.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1600,height:1100});
 page.on('pageerror',e=>errors.push(String(e)));
 page.on('console',m=>{if(/THREE.*(binding|bone|track)|No target node found/i.test(m.text()))errors.push(m.text());});
 page.on('websocket',s=>{s.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m);});s.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;if(m.kind==='state_update')updates.push(m);});});
 const canvas=()=>page.locator('#world-canvas');
 const motion=()=>canvas().evaluate((n,id)=>JSON.parse(n.dataset.dungeonMotions).find(m=>m.id===id),frame.observer_actor_id);
 const pose=(clips,timeout=30000)=>page.waitForFunction(({id,clips})=>clips.includes(JSON.parse(document.querySelector('#world-canvas').dataset.dungeonMotions||'[]').find(m=>m.id===id)?.clip),{id:frame.observer_actor_id,clips},{timeout});
 // Every combat cue naming the observed actor, with the update that carried it.
 const cues=()=>updates.flatMap((u,index)=>(u.events||[])
   .filter(e=>e.kind==='feedback'&&e.cue?.kind==='physical_combat'&&e.cue.target?.actor_id===frame.observer_actor_id)
   .map(e=>({update:index,revision:u.world_revision??null,cue:e.cue})));
 // The relevant set: the configured monster swinging at the controlled defender in
 // the mode that monster actually authored. Anything else is not this scenario, and
 // no_sight/not_ready are excluded because they mean no attack happened.
 const swings=()=>cues().filter(c=>c.cue.source?.actor_id===config.monster
   &&c.cue.mode==='fight'&&SWING_OUTCOMES.has(c.cue.outcome.kind));
 const blocks=()=>swings().filter(c=>c.cue.outcome.kind==='blocked');
 // Bracket the shot: the required clip must hold immediately before AND after the
 // screenshot, so a capture that crossed the transition is rejected instead of
 // silently recorded. This narrows the race; it is not an atomic claim about one
 // rendered frame.
 async function captureBracketed(name,expected,extra){
  const before=await motion();
  await canvas().screenshot({path:`${prefix}-${name}.png`});
  const after=await motion();
  assert(expected.includes(before.clip),`${name}: clip before capture was ${before.clip}, expected one of ${expected}`);
  assert(expected.includes(after.clip),`${name}: clip after capture was ${after.clip}, expected one of ${expected}`);
  captures.push({name,clipBefore:before.clip,clipAfter:after.clip,moving:after.moving,position:frame.observation_center,...extra});
 }
 const requireNoCommands=()=>assert.equal(commands.length,0,`this scenario sends no command; saw ${commands.length}`);

 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
 await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();
 await page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true',undefined,{timeout:45000});
 await waitForWorldPointing(page);
 requireNoCommands();
 // The monster is authored as the expedition's own hold-ground scavenger.
 const hostile=frame.actors.find(a=>a.actor_id===config.monster);
 assert(hostile,`authored monster ${config.monster} must be present`);
 assert.equal(hostile.attack_safety,'open_hostile','the monster must assess the observer as openly hostile');
 assert.deepEqual(hostile.position.position,frame.observation_center.position,'attacker and defender must share the defender\'s tile; Fight is legal only at distance zero');
 // No assertion on the starting clip: the monster's cadence is 1, so its first
 // swing can already have landed by the time the world finishes loading.

 if(negative){
  // Bounded window. The control is only meaningful if the same path really swung.
  await page.waitForTimeout(20000);
  const all=swings();
  const blocked=blocks();
  const outcomes=[...new Set(all.map(s=>s.cue.outcome.kind))].sort();
  assert(all.length>0,`the negative control must still observe real swings from ${config.monster}`);
  assert.deepEqual(blocked,[],`an occupied right hand must produce no martial hand block; observed ${JSON.stringify(outcomes)}`);
  requireNoCommands();
  await captureBracketed('incoming-unblocked',NOT_BLOCKING,{
   outcome:'no block observed',outcomes,swings:all.length});
  assert.deepEqual(errors,[]);
  await writeFile(`${config.output}/${config.engine}-${config.scenario}.json`,JSON.stringify({
   verdict:'PASS',renderer:session.renderer,sex:config.sex,occupiedHand:true,
   guarantee:'the configured monster produced real swings at the controlled defender and none was reported blocked',
   incomingSwings:all.length,outcomes,blockedCount:0,
   commandsSent:commands.length,captures,errors},null,2));
  console.log(`PASS ${config.engine}/${config.scenario} (negative control: ${all.length} swings from ${config.monster}, outcomes ${outcomes.join('/')}, no block)`);
 }else{
  // Wait until the server's own attack is reported blocked.
  const deadline=Date.now()+30000;
  while(Date.now()<deadline&&blocks().length===0)await page.waitForTimeout(200);
  const observed=blocks();
  assert(observed.length>0,`no blocked incoming attack observed; swings=${JSON.stringify(swings().map(s=>s.cue.outcome.kind))}`);
  const last=observed.at(-1);
  // Then wait for incoming-block playback and bracket the shot around it.
  await pose([BLOCK_CLIP],20000);
  await captureBracketed('block',[BLOCK_CLIP],{
   observedBlocks:observed.length,blockRevision:last.revision,blockUpdateIndex:last.update,
   outcome:last.cue.outcome,source:last.cue.source.actor_id,mode:last.cue.mode});
  await pose(['guard'],20000);
  await captureBracketed('guard-after-block',['guard'],{observedBlocks:observed.length});
  requireNoCommands();
  // The defender is unarmed, unarmoured and holds no shield, so the martial hand is
  // the only candidate that could have produced this block.
  assert.equal(frame.character.identity.current_class_id,'martial_artist');
  const rightHand=(frame.carried?.items||[]).find(i=>i.position==='right_hand');
  assert.equal(rightHand,undefined,'the blocking hand must be empty');
  assert.deepEqual(errors,[]);
  await writeFile(`${config.output}/${config.engine}-${config.scenario}.json`,JSON.stringify({
   verdict:'PASS',renderer:session.renderer,sex:config.sex,occupiedHand:false,
   guarantee:'the configured monster produced real swings at the controlled defender, at least one was reported blocked, and incoming-block playback was observed with the clip bracketing the capture. The wire outcome names no block source and the client exposes no cue-to-clip identity, so this is not frame-level attribution: the martial-hand conclusion rests on the fixture being unarmed, unarmoured and shieldless, together with the occupied-hand control',
   defenderClass:frame.character.identity.current_class_id,rightHandEmpty:true,
   incomingSwings:swings().length,observedBlocks:observed.length,
   blockedMode:last.cue.mode,blockRevision:last.revision,blockUpdateIndex:last.update,
   commandsSent:commands.length,captures,errors},null,2));
  console.log(`PASS ${config.engine}/${config.scenario} (attacker=${hostile.actor_id}, ${observed.length} blocked of ${swings().length} swings, clip=${BLOCK_CLIP})`);
 }
} catch(error){await page?.screenshot({path:`${prefix}-failure.png`}).catch(()=>{});throw error;}finally{await session.stop();}
