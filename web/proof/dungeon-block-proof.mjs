// The observed character is the DEFENDER. This is the inverse of the motion proof:
// no command is sent to attack. An authored hold-ground monster shares the tile
// and the server's own automatic attack path produces the swing.
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
const negative=config.occupied===true;
try{
 const context=session.context||await session.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1600,height:1100});
 page.on('pageerror',e=>errors.push(String(e)));
 page.on('console',m=>{if(/THREE.*(binding|bone|track)|No target node found/i.test(m.text()))errors.push(m.text());});
 page.on('websocket',s=>{s.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m);});s.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;if(m.kind==='state_update')updates.push(m);});});
 const canvas=()=>page.locator('#world-canvas');
 const motion=()=>canvas().evaluate((n,id)=>JSON.parse(n.dataset.dungeonMotions).find(m=>m.id===id),frame.observer_actor_id);
 const pose=(clips,timeout=30000)=>page.waitForFunction(({id,clips})=>clips.includes(JSON.parse(document.querySelector('#world-canvas').dataset.dungeonMotions||'[]').find(m=>m.id===id)?.clip),{id:frame.observer_actor_id,clips},{timeout});
 async function capture(name,extra){await canvas().screenshot({path:`${prefix}-${name}.png`});captures.push({name,motion:await motion(),position:frame.observation_center,...extra});}
 // Every combat cue naming the observed actor, with the update index that carried it.
 const cues=()=>updates.flatMap((u,index)=>(u.events||[])
   .filter(e=>e.kind==='feedback'&&e.cue?.kind==='physical_combat'&&e.cue.target?.actor_id===frame.observer_actor_id)
   .map(e=>({update:index,cue:e.cue})));
 const incoming=()=>cues().filter(c=>c.cue.source?.actor_id&&c.cue.source.actor_id!==frame.observer_actor_id);

 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
 await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();
 await page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true',undefined,{timeout:45000});
 await waitForWorldPointing(page);
 assert.equal(commands.length,0,'this scenario sends no command; the monster attacks on its own');
 // The monster is authored as the expedition's own hold-ground scavenger.
 const hostile=frame.actors.find(a=>a.actor_id!=='player'&&a.actor_id===config.monster);
 assert(hostile,`authored monster ${config.monster} must be present`);
 assert.equal(hostile.attack_safety,'open_hostile','the monster must assess the observer as openly hostile');
 assert.deepEqual(hostile.position.position,frame.observation_center.position,'attacker and defender must share the defender\'s tile; Fight is legal only at distance zero');
 // No assertion on the starting clip: the monster's cadence is 1, so its first
 // swing can already have landed by the time the world finishes loading.

 if(negative){
  // Bounded window. The control is only meaningful if the same path really swung.
  await page.waitForTimeout(20000);
  const swings=incoming();
  assert(swings.length>0,'the negative control must still observe incoming attacks');
  const blocked=swings.filter(s=>s.cue.outcome.kind==='blocked');
  const outcomes=[...new Set(swings.map(s=>s.cue.outcome.kind))].sort();
  assert.deepEqual(blocked,[],`an occupied right hand must produce no martial hand block; observed ${JSON.stringify(outcomes)}`);
  const observed=await motion();
  await capture('incoming-unblocked',{outcome:'no block observed',outcomes,swings:swings.length});
  assert.deepEqual(errors,[]);
  await writeFile(`${config.output}/${config.engine}-${config.scenario}.json`,JSON.stringify({
   verdict:'PASS',renderer:session.renderer,sex:config.sex,occupiedHand:true,
   incomingSwings:swings.length,outcomes,blockedCount:0,observedClip:observed.clip,
   commandsSent:commands.length,captures,errors},null,2));
  console.log(`PASS ${config.engine}/${config.scenario} (negative control: ${swings.length} swings, no block)`);
 }else{
  // Wait for the server's own attack to be blocked, tied to that specific cue.
  let blockedCue=null;
  const deadline=Date.now()+30000;
  while(Date.now()<deadline){
   const hit=incoming().find(c=>c.cue.outcome.kind==='blocked');
   if(hit){blockedCue=hit;break;}
   await page.waitForTimeout(200);
  }
  assert(blockedCue,`no blocked incoming attack observed; cues=${JSON.stringify(cues().map(c=>c.cue.outcome.kind))}`);
  assert.equal(blockedCue.cue.mode,'fight','the authored monster attacks with fight');
  // Tie the capture to this cue: wait until the incoming-block clip is on screen.
  // The clip runs for min(clip.duration, 1.5s), so the shot is taken the moment the
  // clip is reported and the reported clip is asserted at that instant, rather than
  // after a sleep that could outlast a short clip and silently capture the guard
  // pose instead. An earlier animation cannot satisfy this: the cue names this
  // attack, and the clip must be the incoming-block one at capture time.
  await pose([BLOCK_CLIP],20000);
  const during=await motion();
  assert.equal(during.clip,BLOCK_CLIP,'the capture must land while the incoming-block clip is playing');
  assert.equal(during.moving,false,'a blocked defender does not travel');
  await capture('block',{clip:during.clip,fromUpdate:blockedCue.update,
   outcome:blockedCue.cue.outcome,source:blockedCue.cue.source.actor_id,mode:blockedCue.cue.mode});
  await pose(['guard'],20000);
  const after=await motion();
  assert.equal(after.clip,'guard','the defender must return to the ordinary stance');
  await capture('guard-after-block',{clip:after.clip});
  // The defender is unarmed, unarmoured and holds no shield, so the martial hand is
  // the only candidate that could have produced this block.
  assert.equal(frame.character.identity.current_class_id,'martial_artist');
  const rightHand=(frame.carried?.items||[]).find(i=>i.position==='right_hand');
  assert.equal(rightHand,undefined,'the blocking hand must be empty');
  assert.deepEqual(errors,[]);
  await writeFile(`${config.output}/${config.engine}-${config.scenario}.json`,JSON.stringify({
   verdict:'PASS',renderer:session.renderer,sex:config.sex,occupiedHand:false,
   defenderClass:frame.character.identity.current_class_id,rightHandEmpty:true,
   incomingSwings:incoming().length,blockedMode:blockedCue.cue.mode,
   blockedFromUpdate:blockedCue.update,duringClip:during.clip,afterClip:after.clip,
   commandsSent:commands.length,captures,errors},null,2));
  console.log(`PASS ${config.engine}/${config.scenario} (attacker=${hostile.actor_id}, blocked, clip=${during.clip})`);
 }
} catch(error){await page?.screenshot({path:`${prefix}-failure.png`}).catch(()=>{});throw error;}finally{await session.stop();}
