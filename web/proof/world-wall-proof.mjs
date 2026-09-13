import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {collectGraphicsErrors} from './graphics-errors.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const session=await launchProofBrowser({name:config.engine,engine:PROOF_ENGINES[config.engine],trustedAuthority:config.authority});
const prefix=`${config.output}/${config.engine}-${config.scenario}`,errors=[],commands=[],captures=[];let page,frame;
try{
 const context=session.context||await session.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1280,height:800});
 collectGraphicsErrors(page,errors);page.on('pageerror',e=>errors.push(String(e)));
 page.on('websocket',s=>{
  s.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m.intent);});
  s.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;});
 });
 const ready=()=>page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true',undefined,{timeout:45000});
 const capture=async name=>{
  await page.waitForFunction(id=>{
   const m=JSON.parse(document.querySelector('#world-canvas').dataset.worldMotions).find(m=>m.id===id);
   return m.clip==='guard'&&m.elapsedMs>500&&m.weights.length===1;
  },frame.observer_actor_id);
  const sample=await page.locator('#world-canvas').evaluate((n,id)=>({
   motion:JSON.parse(n.dataset.worldMotions).find(m=>m.id===id),
   obstruction:JSON.parse(n.dataset.worldOcclusionActors).find(m=>m.id===id),
   png:n.toDataURL('image/png'),
  }),frame.observer_actor_id);
  assert.equal(sample.motion.body,config.sex);assert(sample.obstruction.joints>50,'Actual martial rig joints must participate');
  assert(sample.obstruction.groups.length>0,'The forge wall obscuring the martial limb must fade');
  assert.equal(frame.observation_center.level,'forge');assert.deepEqual(frame.observation_center.position,{x:0,y:2});
  await writeFile(`${prefix}-${name}.png`,Buffer.from(sample.png.split(',')[1],'base64'));delete sample.png;captures.push({name,...sample});
 };
 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);await page.locator('#login').click();await page.locator('#enter').click();await ready();
 const position=structuredClone(frame.observation_center);await capture('guard');
 await page.locator('#reconnect').click();await ready();assert.deepEqual(frame.observation_center,position);await capture('reconnected');
 await page.locator('#logout').click();await page.waitForFunction(()=>document.body.dataset.phase==='signed_out');
 assert.equal(await page.locator('#world-canvas').getAttribute('data-world-occlusion-actors'),null);assert.deepEqual(commands,[]);assert.deepEqual(errors,[]);
 await writeFile(`${prefix}.json`,JSON.stringify({verdict:'PASS',engine:config.engine,sex:config.sex,captures,commands,errors,reconnect:true,cleared:true},null,2)+'\n');console.log(`PASS ${config.engine}/${config.scenario}`);
}catch(error){await page?.screenshot({path:`${prefix}-failure.png`}).catch(()=>{});await writeFile(`${prefix}-failure.json`,JSON.stringify({error:String(error),errors,frame},null,2));throw error;}
finally{await session.stop();}
