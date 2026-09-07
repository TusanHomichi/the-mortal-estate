import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { OrthographicCamera, Vector3 } from 'three';
import { launchProofBrowser, proofBrowsers } from './serve.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;
const config=JSON.parse(input);input='';await mkdir(config.output,{recursive:true});
const reports=[];
for(const spec of proofBrowsers()){
 const launch=await launchProofBrowser(spec);let page;
 const commands=[],previews=[],errors=[];let frame;
 try{
  page=await launch.browser.newPage({viewport:{width:1280,height:1000}});await page.bringToFront();
  page.on('pageerror',error=>errors.push(error.message));
  page.on('websocket',socket=>{
   socket.on('framesent',event=>{const v=JSON.parse(String(event.payload));if(v.kind==='command')commands.push(v);});
   socket.on('framereceived',event=>{const v=JSON.parse(String(event.payload));if(v.frame)frame=v.frame;if(v.kind==='path_preview_result')previews.push(v);});
  });
  const wait=(fn,arg)=>page.waitForFunction(fn,arg,{polling:30,timeout:45000});
  const canvas=page.locator('#world-canvas');
  const ready=()=>wait(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false');
  const here=()=>structuredClone(frame.observation_center);
  const point=async target=>{
   const box=await canvas.boundingBox();assert(box);
   const height=9*Math.sin(Math.PI/4),camera=new OrthographicCamera(-height*1.5/2,height*1.5/2,height/2,-height/2,.1,100);
   const center=here().level==='temple'?{x:3,y:3.5}:here().position;
   const focus=new Vector3(center.x,1.22,center.y);camera.position.copy(focus).add(new Vector3(0,8,8));camera.lookAt(focus);camera.updateMatrixWorld(true);
   const p=new Vector3(target.x,0,target.y).project(camera);
   // Use the visible portion of a partially clipped edge square.
   return {x:Math.max(box.x+10,Math.min(box.x+box.width-10,box.x+(p.x+1)*box.width/2)),
    y:Math.max(box.y+10,Math.min(box.y+box.height-10,box.y+(1-p.y)*box.height/2))};
  };
  const click=async(target,options)=>{await canvas.scrollIntoViewIfNeeded();const p=await point(target);await page.mouse.click(p.x,p.y,options);};
  const draft=async target=>{await ready();await click(target);await wait(()=>document.querySelector('#world-canvas').dataset.walkState==='draft');
   const route=JSON.parse(await canvas.getAttribute('data-walk-route'));assert.deepEqual(route.at(-1),{i:target.x,j:target.y});return route;};
  const completed=async count=>{
   await wait(()=>document.querySelector('#world-canvas').dataset.canAct==='false');
   assert.equal(commands.length,count+1);await ready();assert.equal(commands.length,count+1);
  };
  const move=async target=>{const count=commands.length;await draft(target);await click(target);await completed(count);};
  await page.goto(config.origin+'/index.html');await wait(()=>document.body.dataset.playReady==='true');
  await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
  await page.getByRole('button',{name:'Sign in',exact:true}).click();await wait(()=>document.body.dataset.phase==='selecting');
  await page.locator('#character').selectOption({label:config.character});await page.getByRole('button',{name:'Enter world',exact:true}).click();await ready();
  assert.equal(await canvas.getAttribute('data-presentation'),'first-expedition-study');
  const initial=here();assert.equal(initial.level,'arrival');
  const start=initial.position;
  const open=new Set(frame.tiles.filter(t=>t.passable===true&&!t.transition).map(t=>`${t.position.x},${t.position.y}`));
  const direction=[[0,1],[1,0],[-1,0],[0,-1]].find(([dx,dy])=>[1,2,3].every(n=>open.has(`${start.x+dx*n},${start.y+dy*n}`)));
  assert(direction,'Need three observed open squares for this proof');
  const target={x:start.x+direction[0]*3,y:start.y+direction[1]*3};
  const seq=await canvas.getAttribute('data-sequence');
  await draft(target);await page.keyboard.press('Escape');assert.equal(await canvas.getAttribute('data-walk-state'),'idle');
  await draft(target);await click(target,{button:'right'});assert.equal(await canvas.getAttribute('data-walk-state'),'idle');
  const previewCount=previews.length;await draft(target);
  await new Promise((resolve,reject)=>{const end=Date.now()+10000;const tick=()=>previews.length>previewCount?resolve():Date.now()>end?reject(Error('No server preview')):setTimeout(tick,20);tick();});
  assert.equal(commands.length,0);assert.deepEqual(here(),initial);assert.equal(await canvas.getAttribute('data-sequence'),seq);
  assert.equal(previews.at(-1).preview.accepted_steps,'3');
  await canvas.screenshot({path:`${config.output}/${spec.name}-exterior-draft.png`});
  await click(target);await wait(()=>document.querySelector('#world-canvas').dataset.canAct==='false');
  assert.equal(commands.length,1);assert.equal(commands[0].intent.path.length,3);
  await click(start);await page.keyboard.press('Escape');await click(start,{button:'right'});
  assert.equal(commands.length,1);assert.equal(await canvas.getAttribute('data-walk-cursor'),'waiting');
  await canvas.screenshot({path:`${config.output}/${spec.name}-committed.png`});
  await ready();assert.deepEqual(here().position,target);
  // A real native double-click, with the server assessment in flight.
  await click(start,{clickCount:2,delay:60});await completed(1);assert.deepEqual(here(),initial);
  await draft(target);await page.getByRole('button',{name:'Reconnect',exact:true}).click();await ready();
  assert.equal(await canvas.getAttribute('data-walk-state'),'idle');assert.deepEqual(here(),initial);
  if(config.temple){
   assert.deepEqual(start,{x:6,y:8});
   await move({x:6,y:7});assert.equal(here().level,'temple');
   assert.equal(await canvas.getAttribute('data-walk-state'),'idle');
   await draft({x:3,y:3});await canvas.screenshot({path:`${config.output}/${spec.name}-temple-draft.png`});
   const count=commands.length;await click({x:3,y:3});await completed(count);assert.deepEqual(here().position,{x:3,y:3});
   await move({x:3,y:6});await move({x:3,y:7});assert.deepEqual(here(),initial);
  }
  await page.getByRole('button',{name:'Sign out',exact:true}).click();await wait(()=>document.body.dataset.phase==='signed_out');
  assert.deepEqual(errors,[]);
  reports.push({engine:spec.name,verdict:'PASS',first_click_nonmutating:true,second_click_one_command:true,native_double_click:true,
   escape_and_right_click_cancel:true,cooldown_input_locked:true,reconnect_discards_draft:true,temple_transition:!!config.temple,
   restored_initial_position:true,commands:commands.length,server_previews:previews.length});
  console.log('PASS authoritative footprint controls',spec.name);
 }catch(error){
  await page?.screenshot({path:`${config.output}/${spec.name}-failure.png`}).catch(()=>{});
  await writeFile(`${config.output}/${spec.name}-failure.json`,JSON.stringify({error:String(error),errors,position:frame?.observation_center,
   commands:commands.map(c=>c.intent),previews:previews.map(p=>({disposition:p.disposition,preview:p.preview}))},null,2));throw error;
 }finally{await launch.stop();}
}
await writeFile(`${config.output}/path-controls.json`,JSON.stringify({verdict:'PASS',reports},null,2)+'\n');
