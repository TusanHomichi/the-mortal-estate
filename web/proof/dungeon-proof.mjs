import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {worldCellPoint,waitForWorldPointing} from './world-pointing.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const launched=await launchProofBrowser({name:config.engine,engine:PROOF_ENGINES[config.engine],trustedAuthority:config.authority});
let page,frame;const commands=[],results=[],errors=[],captures=[];
try {
 const context=launched.context||await launched.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1600,height:1100});
 page.on('pageerror',e=>errors.push(String(e)));page.on('websocket',socket=>{
  socket.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m);});
  socket.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;if(m.kind==='command_result')results.push(m);});
 });
 const ready=()=>page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false',undefined,{timeout:45000});
 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
 await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();await ready();
 async function accepted(before){
  await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.canAct==='false');await ready();
  assert.equal(commands.length,before+1);const result=results.find(r=>r.command_id===commands.at(-1).command_id);assert(result);assert.equal(result.disposition.kind,'accepted');
 }
 async function move(x,y){
  await ready();const p=await worldCellPoint(page,{x,y}),before=commands.length;await page.mouse.click(p.x,p.y);
  await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.walkState==='draft');
  await page.mouse.click(p.x,p.y);await accepted(before);assert.deepEqual(frame.observation_center.position,{x,y});return commands.at(-1).intent;
 }
 async function traverse(direction,expected){
  await ready();assert(frame.action_options.some(a=>a.enabled&&a.intent?.kind==='traverse'&&a.intent.traversal===direction));
  const before=commands.length,p=await worldCellPoint(page,frame.observation_center.position);
  await page.mouse.click(p.x,p.y,{clickCount:2,delay:60});await accepted(before);
  assert.equal(commands.at(-1).intent.kind,'traverse');assert.deepEqual(frame.observation_center,expected);
 }
 async function capture(name){
  await waitForWorldPointing(page);const canvas=page.locator('#world-canvas');
  assert.equal(await canvas.getAttribute('data-presentation-error'),null);
  if(frame.observation_center.level.startsWith('d')){
   assert.equal(await canvas.getAttribute('data-presentation'),'dungeon-3d');const view=JSON.parse(await canvas.getAttribute('data-dungeon-view'));
   assert.equal(view.cells,7);assert.equal(view.elevation,55);assert.equal(view.fieldOfView,20);
   const expected=frame.tiles.filter(t=>t.terrain_id&&Math.abs(t.position.x-view.center.x)<=3&&Math.abs(t.position.y-view.center.y)<=3);
   assert.deepEqual(view.tiles.map(t=>t.position),expected.map(t=>t.position));
   assert(view.walls.every(w=>expected.some(t=>t.position.x===w.tile.x&&t.position.y===w.tile.y)));
   const anchors=JSON.parse(await canvas.getAttribute('data-dungeon-actor-anchors'));
   const occupants=frame.actors.filter(a=>a.position.position.x===view.center.x&&a.position.position.y===view.center.y&&a.life_state!=='dead');
   if(occupants.length===1){const self=anchors.find(a=>a.id===frame.observer_actor_id);assert(self);assert(Math.abs(self.x-view.center.x)<1e-9&&Math.abs(self.y-view.center.y)<1e-9);}
   const black=await canvas.evaluate(node=>{const c=document.createElement('canvas');c.width=node.width;c.height=node.height;const x=c.getContext('2d');x.drawImage(node,0,0);return [...x.getImageData(2,2,1,1).data];});assert.deepEqual(black,[0,0,0,255]);
  }else assert.equal(await canvas.getAttribute('data-presentation'),'pixel-art');
  await canvas.screenshot({path:`${config.output}/${config.engine}-${config.scenario}-${name}.png`});captures.push({name,location:frame.observation_center});
 }
 await capture('start');
 if(config.scenario==='doors'){
  const door=()=>frame.tiles.find(t=>t.position.x===22&&t.position.y===9)?.transition;
  assert.equal(door()?.door_open,false);await move(22,9);assert.equal(door()?.door_open,true);await move(21,9);
  assert.deepEqual(await move(24,9),{kind:'move_path',path:['east','east','east']});await capture('open-door-sprint');
  await move(24,8);await move(25,7);await traverse('stairs_up',{realm:'first_expedition',level:'temple',position:{x:0,y:5}});await capture('temple');
  await move(0,4);await traverse('stairs_down',{realm:'first_expedition',level:'d1_entry',position:{x:24,y:7}});await capture('returned');
  for(const viewport of [{width:1920,height:1080},{width:3840,height:2160},{width:390,height:844}]){await page.setViewportSize(viewport);await capture(`size-${viewport.width}`);}
 }else{
  await traverse('stairs_down',config.down);await capture('lower-floor');
  await move(config.returnAt.x,config.returnAt.y);await traverse('stairs_up',config.up);await capture('returned');
 }
 const before=frame.observation_center;await page.getByRole('button',{name:'Reconnect',exact:true}).click();await ready();assert.deepEqual(frame.observation_center,before);await capture('reconnected');
 assert.deepEqual(errors,[]);await page.getByRole('button',{name:'Sign out',exact:true}).click();await page.waitForFunction(()=>document.body.dataset.phase==='signed_out');
 assert.equal(await page.locator('#world-canvas').getAttribute('data-study-level'),null);
 await writeFile(`${config.output}/${config.engine}-${config.scenario}.json`,JSON.stringify({verdict:'PASS',renderer:launched.renderer,scenario:config.scenario,captures,commands:commands.map(c=>c.intent),errors},null,2));
 console.log(`PASS ${config.engine}/${config.scenario}`);
}catch(error){await page?.screenshot({path:`${config.output}/${config.engine}-${config.scenario}-failure.png`}).catch(()=>{});await writeFile(`${config.output}/${config.engine}-${config.scenario}-failure.json`,JSON.stringify({error:String(error),errors,frame,lastCommand:commands.at(-1),lastResult:results.at(-1)},null,2));throw error;}finally{await launched.stop();}
