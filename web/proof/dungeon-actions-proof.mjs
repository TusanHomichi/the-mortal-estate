import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {waitForWorldPointing} from './world-pointing.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const session=await launchProofBrowser({name:config.engine,engine:PROOF_ENGINES[config.engine],trustedAuthority:config.authority});
let page,frame;const commands=[],errors=[];
try{
 const context=session.context||await session.browser.newContext();page=await context.newPage();await page.setViewportSize({width:1600,height:1100});
 page.on('pageerror',e=>errors.push(String(e)));page.on('websocket',s=>{s.on('framesent',e=>{const m=JSON.parse(String(e.payload));if(m.kind==='command')commands.push(m);});s.on('framereceived',e=>{const m=JSON.parse(String(e.payload));if(m.frame)frame=m.frame;});});
 await page.goto(config.origin+'/');await page.waitForFunction(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();
 const ready=()=>page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false');await ready();await waitForWorldPointing(page);
 const offered=frame.action_options.find(a=>a.enabled&&a.intent?.kind==='traverse'&&a.intent.traversal==='stairs_up');assert(offered);const expected=structuredClone(offered.intent);
 const point=await page.locator('#world-canvas').evaluate((node,id)=>{const p=JSON.parse(node.dataset.dungeonActorPoints).find(p=>p.id===id);if(!p)throw Error("Player body missing");const b=node.getBoundingClientRect();return {x:b.left+p.px*b.width,y:b.top+p.py*b.height};},frame.observer_actor_id);
 await page.mouse.click(point.x,point.y,{button:'right'});const dialog=page.locator('.resident-dialog[open]');await dialog.waitFor();
 assert.equal(commands.length,0,'opening actor menu is not a command');await dialog.locator(`button[data-action=${JSON.stringify(`character/${offered.id}`)}]`).click();
 await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.canAct==='false');await ready();assert.deepEqual(commands.at(-1).intent,expected);assert.equal(frame.observation_center.level,'d3');assert.deepEqual(frame.observation_center.position,{x:10,y:17});
 if(await dialog.count())await dialog.getByRole('button',{name:'Close',exact:true}).click();
 await page.screenshot({path:`${config.output}/${config.engine}-actor-stair.png`});assert.deepEqual(errors,[]);
 const lost=await page.locator('#world-canvas').evaluate(node=>{const ext=node.getContext('webgl2').getExtension('WEBGL_lose_context');if(!ext)return false;ext.loseContext();return true;});
 assert(lost,'context-loss capability required');await page.getByRole('alert').filter({hasText:'Dungeon graphics were lost'}).waitFor();
 assert.equal(await page.locator('#world-canvas').getAttribute('data-dungeon-view'),null);
 assert.equal(await page.locator('#world-canvas').getAttribute('data-presentation'),'dungeon-3d');
 await page.getByRole('button',{name:'Sign out',exact:true}).click();await page.waitForFunction(()=>document.body.dataset.phase==='signed_out');
 for(const failure of ['missing','digest']){
  await page.route('**/feel-assets/dungeon-adventurer.glb',route=>route.fulfill({status:failure==='missing'?404:200,body:'not the bound body'}));
  await page.reload();await page.waitForFunction(()=>document.body.dataset.playReady==='failed');assert(await page.locator('#login').isDisabled());
  await page.unroute('**/feel-assets/dungeon-adventurer.glb');
 }
 await writeFile(`${config.output}/${config.engine}-actor-actions.json`,JSON.stringify({verdict:'PASS',renderer:session.renderer,offeredActorStairPreserved:true,missingBodyRefused:true,digestMismatchRefused:true,contextLossClearsAndExplains:true},null,2));console.log(`PASS ${config.engine}/actor-actions`);
} catch(error){await page?.screenshot({path:`${config.output}/${config.engine}-actor-actions-failure.png`}).catch(()=>{});throw error;}finally{await session.stop();}
