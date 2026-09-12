import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {worldCellPoint,waitForWorldPointing} from './world-pointing.mjs';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const engine=PROOF_ENGINES[config.engine];assert(engine);assert(['male','female'].includes(config.body));
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
let page,frame;const commands=[],errors=[],captures=[];
try {
 const context=launched.context||await launched.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1280,height:800});
 page.on('pageerror',e=>errors.push(e.message));page.on('websocket',socket=>{
  socket.on('framesent',e=>{const v=JSON.parse(String(e.payload));if(v.kind==='command')commands.push(v);});
  socket.on('framereceived',e=>{const v=JSON.parse(String(e.payload));if(v.frame)frame=v.frame;});
 });
 const wait=(fn,arg)=>page.waitForFunction(fn,arg,{timeout:45000,polling:30});
 const ready=()=>wait(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false');
 const canvas=page.locator('#world-canvas');
 const mark=async name=>{
  await waitForWorldPointing(page);const motions=JSON.parse(await canvas.getAttribute('data-world-motions'));
  assert.equal(motions.find(m=>m.id===frame.observer_actor_id)?.body,config.body);
  assert.equal(await canvas.getAttribute('data-presentation'),'world-3d');
  await canvas.screenshot({path:`${config.output}/${config.engine}-${config.body}-${name}.png`});captures.push(name);
 };
 const move=async(target,checkWalk=false)=>{
  await ready();const count=commands.length,p=await worldCellPoint(page,target);await page.mouse.click(p.x,p.y,{clickCount:2,delay:60});
  await wait(()=>document.querySelector('#world-canvas').dataset.canAct==='false');assert.equal(commands.length,count+1);
  if(checkWalk){
   const motion=JSON.parse(await canvas.getAttribute('data-world-motions')).find(m=>m.id===frame.observer_actor_id);
   assert.equal(motion.clip,'walk');assert.equal(motion.moving,true);await mark('walking');
  }
  await ready();assert.equal(commands.length,count+1);
 };
 await page.goto(config.origin+'/');await wait(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);await page.locator('#login').click();
 await wait(()=>document.body.dataset.phase==='selecting');await page.locator('#enter').click();await ready();assert.equal(frame.observation_center.level,'temple');
 await mark('temple');await move({x:3,y:5},true);await move({x:3,y:7});assert.equal(frame.observation_center.level,'arrival');await mark('town');
 await move({x:13,y:9},true);await move({x:13,y:7});assert.equal(frame.observation_center.level,'temple');await mark('returned');
 const before=structuredClone(frame.observation_center);await page.locator('#reconnect').click();await ready();assert.deepEqual(frame.observation_center,before);await mark('reconnected');
 await page.locator('#logout').click();await wait(()=>document.body.dataset.phase==='signed_out');assert.equal(await canvas.getAttribute('data-study-actor-count'),'0');assert.deepEqual(errors,[]);
 await writeFile(`${config.output}/${config.engine}-world-figure.json`,JSON.stringify({verdict:'PASS',engine:config.engine,body:config.body,
  captures,commands:commands.length,walk:true,temple_town_return:true,reconnect:true,logout_clears:true},null,2)+'\n');
}catch(error){await page?.screenshot({path:`${config.output}/${config.engine}-figure-failure.png`}).catch(()=>{});throw error;}
finally{await launched.stop();}
