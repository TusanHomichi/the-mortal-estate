import assert from 'node:assert/strict';
import {writeFile} from 'node:fs/promises';

/** Sample real framebuffer draws, never a frozen pose or a remote screenshot race. */
export async function armPhases(page,id,clips,phases){
 await page.evaluate(({id,clips,phases})=>{
  const canvas=document.querySelector('#world-canvas');let active=false;
  window.martialPhaseObserver?.disconnect();window.martialPhases=[];
  const observer=new MutationObserver(()=>{
   const motion=JSON.parse(canvas.dataset.worldMotions).find(m=>m.id===id);
   const solo=motion.weights.length===1&&Math.abs(motion.weights[0].weight-1)<1e-5;
   if(clips.includes(motion.clip))active=true;
   if(!active||!solo)return;
   for(const phase of phases){
    if(window.martialPhases.some(p=>p.name===phase.name))continue;
    const matches=phase.settled?motion.clip==='guard'&&motion.elapsedMs>=150:
     clips.includes(motion.clip)&&motion.phase>=phase.min&&motion.phase<phase.max;
    if(!matches)continue;
    window.martialPhases.push({name:phase.name,motion,
     anchor:JSON.parse(canvas.dataset.worldActorAnchors).find(a=>a.id===id),
     view:JSON.parse(canvas.dataset.worldView),occlusion:Number(canvas.dataset.worldOcclusion),png:canvas.toDataURL('image/png')});
   }
   if(window.martialPhases.length===phases.length)observer.disconnect();
  });
  window.martialPhaseObserver=observer;observer.observe(canvas,{attributes:true,attributeFilter:['data-world-motions']});
 },{id,clips,phases});
}

export async function savePhases(page,prefix,phases){
 await page.waitForFunction(count=>window.martialPhases?.length===count,phases.length,{timeout:15000});
 const samples=await page.evaluate(()=>window.martialPhases);
 for(const sample of samples){
  assert.equal(sample.motion.weights.length,1);assert.equal(sample.motion.weights[0].clip,sample.motion.clip);
  await writeFile(`${prefix}-${sample.name}.png`,Buffer.from(sample.png.split(',')[1],'base64'));delete sample.png;
 }
 return samples;
}
