import assert from 'node:assert/strict';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
import {build} from 'vite';
import {startVite,launchProofBrowser,PROOF_ENGINES,webRoot} from './serve.mjs';
import {installPixelProfile,beginPixelProfile,finishPixelProfile} from './pixel-profile.mjs';

const [packetPath,outputPath,onlyEngine]=process.argv.slice(2);
if(!packetPath||!outputPath||onlyEngine&&!PROOF_ENGINES[onlyEngine])throw Error('Usage: pixel-crowd-proof.mjs EXTERNAL_PACKET EXTERNAL_OUTPUT [ENGINE]');
const output=path.resolve(outputPath),root=path.dirname(webRoot);
for(const candidate of [output,path.resolve(packetPath)]) {
 const relative=path.relative(root,candidate);
 assert(relative==='..'||relative.startsWith(`..${path.sep}`)||path.isAbsolute(relative),'Packet and output must be outside checkout');
}
await mkdir(output,{recursive:true});
const receipt=path.join(output,'verification.json');await writeFile(receipt,JSON.stringify({verdict:'INCOMPLETE'})+'\n');
// Build the real renderer once. No development transform or HMR runs in samples.
await build({configFile:false,root:webRoot,build:{outDir:path.join(output,'bundle'),emptyOutDir:true,
  lib:{entry:path.join(webRoot,'proof/pixel-crowd-scene.mjs'),formats:['es'],fileName:()=> 'crowd.js',cssFileName:'crowd'},
  target:'esnext',minify:true}});
const js=await readFile(path.join(output,'bundle/crowd.js'),'utf8'),css=await readFile(path.join(output,'bundle/crowd.css'),'utf8');
const server=await startVite(packetPath),reports=[];
try {
 for(const [name,engine] of Object.entries(PROOF_ENGINES).filter(([name])=>!onlyEngine||name===onlyEngine)) {
  const launched=await launchProofBrowser({name,engine,executablePath:engine.executablePath()});
  try {
   const context=launched.context||await launched.browser.newContext(),page=await context.newPage(),errors=[];
   page.on('pageerror',e=>errors.push(e.message));await installPixelProfile(page);
   await page.route('**/__pixel-crowd__*',route=>{
    const url=new URL(route.request().url());
    if(url.pathname.endsWith('.js'))return route.fulfill({contentType:'application/javascript',body:js});
    if(url.pathname.endsWith('.css'))return route.fulfill({contentType:'text/css',body:css});
    return route.fulfill({contentType:'text/html',body:'<!doctype html><link rel="stylesheet" href="/__pixel-crowd__.css"><body style="margin:0"><canvas id="world-canvas"></canvas><script type="module" src="/__pixel-crowd__.js"></script>'});
   });
   await page.goto(server.baseUrl+'__pixel-crowd__');await page.waitForFunction(()=>window.crowdFixture);
   const cases=[];
   for(const level of ['temple','arrival'])for(const size of [{width:1280,height:800},{width:1920,height:1080}])for(const count of [1,10]) {
    await page.setViewportSize(size);await page.evaluate(({level,count})=>window.crowdFixture.start(level,count),{level,count});
    await page.waitForTimeout(4500);const before=await page.evaluate(()=>window.crowdFixture.state());
    assert.equal(before.detailed,count);assert.equal(before.moving,count);
    await beginPixelProfile(page);await page.waitForTimeout(8000);const profile=await finishPixelProfile(page);
    const after=await page.evaluate(()=>window.crowdFixture.state());
    assert.equal(after.actors,count);assert.equal(after.detailed,count);assert.equal(after.moving,count);
    assert(after.updates>=before.updates+3,'The crowd must keep changing during the sample');
    assert(profile.composition_frames/profile.frames>.9,'Reused idle frames cannot stand in for a moving crowd');
    assert.equal(profile.stages.texture_upload?.calls_per_frame??0,0,'Warmed movement must reuse resident textures');
    assert.equal(profile.stages.texture_update?.calls_per_frame??0,0,'Warmed movement must not transfer replacement pixels');
    assert.equal(profile.stages.canvas_composition?.calls_per_frame??0,0,'Warmed movement must compose on the GPU');
    assert.equal(after.bounds.length,count);
    for(const b of after.bounds)assert(b.x>=0&&b.y>=0&&b.x+b.width<=after.viewport.width&&b.y+b.height<=after.viewport.height,'Every detailed figure must fit on screen');
    const report={level,...size,count,...profile};cases.push(report);
    await page.locator('#world-canvas').screenshot({path:path.join(output,`${name}-${level}-${size.width}-${count}.png`)});
    console.log(name,level,size.width,count,profile.submitted_fps.toFixed(1),profile.draw_p95_ms.toFixed(1));
   }
   assert.deepEqual(errors,[]);await page.evaluate(()=>window.crowdFixture.stop());
   const report={engine:name,adapter:launched.renderer,rendering:launched.rendering,cases};reports.push(report);
   await writeFile(path.join(output,`${name}-crowd.json`),JSON.stringify(report,null,2)+'\n');
  }finally{await launched.stop();}
 }
 await writeFile(receipt,JSON.stringify({verdict:reports.length===3?'PASS':'INSPECTION',
  measurement:'Built product renderer, synthetic presentation inputs, 4.5s warmup and 8s sustained movement; no server, no scanout FPS claim; three shared figure designs and existing four-caster shadow cap',reports},null,2)+'\n');
}finally{await server.stop();}
