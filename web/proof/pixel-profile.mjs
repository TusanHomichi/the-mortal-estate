import assert from 'node:assert/strict';

/** Observe the real renderer callback; browser copies outside it do not count. */
export async function installPixelProfile(page) {
 await page.addInitScript(()=>{
  const state={enabled:false,inside:false,frames:[],stages:{}};window.pixelProfile=state;
  const timed=(prototype,name,key)=>{const original=prototype[name];prototype[name]=function(...args){if(!state.enabled||!state.inside)return original.apply(this,args);const start=performance.now();try{return original.apply(this,args);}finally{const k=typeof key==='function'?key.call(this,args):key;const s=state.stages[k]??={calls:0,ms:0};s.calls++;s.ms+=performance.now()-start;}};};
  timed(CanvasRenderingContext2D.prototype,'drawImage',function(){return this.canvas.id==='world-canvas'?'display_copy':'canvas_composition';});
  const targets=new WeakMap(),bind=WebGLRenderingContext.prototype.bindFramebuffer;
  WebGLRenderingContext.prototype.bindFramebuffer=function(target,buffer){targets.set(this,buffer);return bind.call(this,target,buffer);};
  timed(WebGLRenderingContext.prototype,'texImage2D','texture_upload');timed(WebGLRenderingContext.prototype,'texSubImage2D','texture_update');timed(WebGLRenderingContext.prototype,'drawArrays',function(){return targets.get(this)?'gpu_composition':'gpu_submit';});
  for(const name of ['width','height']){const d=Object.getOwnPropertyDescriptor(HTMLCanvasElement.prototype,name);Object.defineProperty(HTMLCanvasElement.prototype,name,{...d,set(value){const start=performance.now();try{return d.set.call(this,value);}finally{if(state.enabled&&state.inside){const s=state.stages.canvas_resize??={calls:0,ms:0};s.calls++;s.ms+=performance.now()-start;}}}});}
  const raf=requestAnimationFrame;
  window.requestAnimationFrame=function(callback){
   if(!callback.toString().includes('pixelDrawP95Ms'))return raf.call(this,callback);
   return raf.call(this,t=>{const start=performance.now();state.inside=true;const before=state.stages.gpu_submit?.calls??0,composed=(state.stages.canvas_composition?.calls??0)+(state.stages.gpu_composition?.calls??0);try{callback(t);}finally{state.inside=false;if(state.enabled&&(state.stages.gpu_submit?.calls??0)>before)state.frames.push({at:start,ms:performance.now()-start,composed:(state.stages.canvas_composition?.calls??0)+(state.stages.gpu_composition?.calls??0)>composed});}});
  };
 });
}

export async function beginPixelProfile(page) {
 await page.evaluate(()=>{Object.assign(window.pixelProfile,{enabled:true,frames:[],stages:{}});});
}

export async function finishPixelProfile(page) {
   const sample=await page.evaluate(()=>{window.pixelProfile.enabled=false;return {frames:window.pixelProfile.frames,stages:window.pixelProfile.stages};});
   assert(sample.frames.length>20,'Actual renderer callback must be observed');
   const percentile=(values,p)=>[...values].sort((a,b)=>a-b)[Math.floor((values.length-1)*p)];
   const intervals=sample.frames.slice(1).map((f,i)=>f.at-sample.frames[i].at),times=sample.frames.map(f=>f.ms);
   const compositionTimes=sample.frames.filter(f=>f.composed).map(f=>f.ms);
   return {frames:times.length,draw_p50_ms:percentile(times,.5),draw_p95_ms:percentile(times,.95),composition_frames:compositionTimes.length,composition_p95_ms:compositionTimes.length?percentile(compositionTimes,.95):null,interval_p50_ms:percentile(intervals,.5),interval_p95_ms:percentile(intervals,.95),submitted_fps:1000*(times.length-1)/(sample.frames.at(-1).at-sample.frames[0].at),stages:Object.fromEntries(Object.entries(sample.stages).map(([k,v])=>[k,{calls_per_frame:v.calls/times.length,ms_per_frame:v.ms/times.length}]))};
}
