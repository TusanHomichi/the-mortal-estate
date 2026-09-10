import assert from 'node:assert/strict';
import {worldCellPoint} from './world-pointing.mjs';
import {writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
import {installPixelProfile,beginPixelProfile,finishPixelProfile} from './pixel-profile.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;const config=JSON.parse(input);input='';
const engine=PROOF_ENGINES[config.engine];assert(engine);
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
try {
 const context=launched.context||await launched.browser.newContext();const page=await context.newPage();const errors=[];let frame;
 page.on('pageerror',e=>errors.push(e.message));page.on('websocket',s=>s.on('framereceived',e=>{const v=JSON.parse(String(e.payload));if(v.frame)frame=v.frame;}));
 await installPixelProfile(page);
 await page.setViewportSize({width:1280,height:800});await page.goto(config.origin+'/');
 await page.waitForFunction(()=>document.body.dataset.playReady==='true');await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);await page.getByRole('button',{name:'Sign in',exact:true}).click();await page.getByRole('button',{name:'Enter world',exact:true}).click();
 const ready=()=>page.waitForFunction(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false');await ready();
 const canvas=page.locator('#world-canvas');
 const move=async cell=>{await ready();const p=await worldCellPoint(page,cell);await page.mouse.click(p.x,p.y,{clickCount:2,delay:60});await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.canAct==='false');await ready();};
 const reports=[];
 for(const level of ['temple','arrival']){
  assert.equal(frame.observation_center.level,level);
  for(const size of [{width:1280,height:800,motion:false},{width:1920,height:1080,motion:false},{width:1920,height:1080,motion:true}]){
   await page.setViewportSize(size);await page.waitForTimeout(2500);
   await beginPixelProfile(page);
   // No screenshots, pixel reads or resizing during either sample. The motion
   // sample uses real movement commands against the disposable authority.
   if(size.motion){
    for(let i=0;i<4;i++){await move(level==='temple'?{x:2,y:6}:{x:13,y:9});await move(level==='temple'?{x:3,y:6}:{x:13,y:8});}
   } else await page.waitForTimeout(8000);
   const report={level,...size,...await finishPixelProfile(page)};
   reports.push(report);await canvas.screenshot({path:`${config.output}/${config.engine}-${level}-${size.width}${size.motion?'-motion':''}.png`});
  }
  if(level==='temple'){await move({x:3,y:7});assert.equal(frame.observation_center.level,'arrival');}
 }
 assert.deepEqual(errors,[]);
 await canvas.evaluate(c=>{const extension=c.getContext('webgl').getExtension('WEBGL_lose_context');if(!extension)throw Error('Context-loss proof unavailable');extension.loseContext();});
 await page.waitForFunction(()=>document.querySelector('#world-canvas').dataset.pixelEffects==='failed');
 assert.equal(await canvas.getAttribute('data-study-actor-count'),'0');assert.equal(await canvas.getAttribute('data-pixel-projection'),null);
 await page.locator('.pixel-graphics-error:not([hidden])').waitFor();
 await writeFile(`${config.output}/${config.engine}-pixel-performance.json`,JSON.stringify({verdict:'PASS',engine:config.engine,adapter:launched.renderer,rendering:launched.rendering,context_loss_refuses_pointing:true,measurement:'instrumented native renderer submissions; steady 8s and eight-command motion windows after 2.5s warmup, no screenshot/readback/resize during samples; not display-scanout FPS',reports},null,2)+'\n');
}finally{await launched.stop();}
