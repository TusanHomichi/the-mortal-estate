import assert from 'node:assert/strict';
import {writeFile,mkdir} from 'node:fs/promises';
import {startVite,launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
const [output]=process.argv.slice(2);if(!output)throw Error('Usage: world-occlusion-proof.mjs EXTERNAL_OUTPUT');
await mkdir(output,{recursive:true});const server=await startVite(''),reports=[];
try {
 for(const [name,engine] of Object.entries(PROOF_ENGINES)){
  const launched=await launchProofBrowser({name,engine,executablePath:engine.executablePath()});
  try {
   const context=launched.context||await launched.browser.newContext(),page=await context.newPage();
   await page.route('**/__world-occlusion__',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><body></body>'}));
   await page.goto(server.baseUrl+'__world-occlusion__');
   const result=await page.evaluate(async()=>{const {measureOcclusion}=await import('/proof/world-occlusion-scene.mjs');return measureOcclusion();});
   assert.deepEqual(result.opaque,[255,0,0,255]);assert.deepEqual(result.restored,result.opaque);assert.deepEqual(result.cleared,result.opaque);
   for(const [i,expected] of [102,153,0,255].entries())assert(Math.abs(result.faded[i]-expected)<=2,`Overlapping surfaces must blend once: ${result.faded}`);
   const shadow=await page.evaluate(async()=>{const {measureShadowCycle}=await import('/proof/world-occlusion-scene.mjs');return measureShadowCycle();});
   assert(shadow.opaque[0]>100&&shadow.opaque[0]>shadow.opaque[1]+50,JSON.stringify(shadow));assert.deepEqual(shadow.cachedOpaque,shadow.opaque);assert.deepEqual(shadow.restored,shadow.opaque);assert.deepEqual(shadow.cleared,shadow.opaque);
   assert(shadow.faded[0]>0&&shadow.faded[1]>shadow.opaque[1]+50,'Cached shadow fade must retain scenery and the body');assert(shadow.errors.every(e=>e===0),'Cached shadow draws must produce no GPU error');
   const retired=await page.evaluate(async()=>{const {measureShadowCycle}=await import('/proof/world-occlusion-scene.mjs');return measureShadowCycle(2);});
   assert(retired.errors.some(e=>e!==0),'Retired shadow mode must fail the negative control');
   reports.push({engine:name,verdict:'PASS',rendering:launched.rendering,...result,shadow,retired_mode_refused:true});console.log(`PASS ${name}/composed-occlusion`);
  }finally{await launched.stop();}
 }
 await writeFile(`${output}/verification.json`,JSON.stringify({verdict:'PASS',reports},null,2)+'\n');
}finally{await server.stop();}
