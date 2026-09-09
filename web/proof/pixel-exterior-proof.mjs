import assert from 'node:assert/strict';
import {pixelCellPoint} from './pixel-pointing.mjs';
import {readFile,writeFile} from 'node:fs/promises';
import {launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
let input='';for await(const chunk of process.stdin)input+=chunk;
const config=JSON.parse(input);input='';
const engine=PROOF_ENGINES[config.engine];assert(engine);
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
const geography=JSON.parse(await readFile(new URL('../../content/lands/first-expedition/generated/workbench_projection.json',import.meta.url),'utf8'));
let page,frame,stage='startup';const commands=[],results=[],errors=[],captures=[];
try {
 page=await launched.browser.newPage({viewport:{width:1280,height:800}});
 page.on('pageerror',e=>errors.push(e.message));
 page.on('websocket',socket=>{
  socket.on('framesent',e=>{const v=JSON.parse(String(e.payload));if(v.kind==='command')commands.push(v);});
  socket.on('framereceived',e=>{const v=JSON.parse(String(e.payload));if(v.frame)frame=v.frame;if(v.kind==='command_result')results.push(v);});
 });
 const wait=(fn,arg)=>page.waitForFunction(fn,arg,{polling:30,timeout:45000});
 const canvas=page.locator('#world-canvas');
 const ready=()=>wait(()=>document.body.dataset.phase==='playing'&&document.querySelector('#world-canvas').dataset.canAct==='true'&&document.querySelector('#world-canvas').dataset.pending==='false');
 const here=()=>structuredClone(frame.observation_center);
 const click=async(cell,options)=>{const p=await pixelCellPoint(page,cell);await page.mouse.click(p.x,p.y,options);};
 const move=async cell=>{
  await ready();const count=commands.length;
  await click(cell,{clickCount:2,delay:60});
  await wait(()=>document.querySelector('#world-canvas').dataset.canAct==='false');
  assert.equal(commands.length,count+1);await ready();assert.equal(commands.length,count+1);
  assert.equal(results.at(-1).disposition.kind,'accepted');
 };
 const walkTo=async target=>{
  const location=here(),key=p=>`${p.x}:${p.y}`;
  const member=geography.members.find(m=>m.member===location.level),open=new Set(member.cells.filter(c=>c.passable).map(key));
  for(const edge of geography.connectivity.edges.filter(e=>e.from_member===location.level))if(key(edge.from)!==key(target))open.delete(key(edge.from));
  const queue=[location.position],previous=new Map([[key(location.position),null]]);
  for(let i=0;i<queue.length;i++){
   const p=queue[i];if(key(p)===key(target))break;
   for(const [dx,dy]of [[0,-1],[1,0],[0,1],[-1,0]]){const q={x:p.x+dx,y:p.y+dy};if(open.has(key(q))&&!previous.has(key(q))){previous.set(key(q),p);queue.push(q);}}
  }
  assert(previous.has(key(target)));const route=[];
  for(let p=target;previous.get(key(p))!==null;p=previous.get(key(p)))route.unshift(p);
  while(route.length) {
   // Authored connectivity plans the walk; the current observation bounds each
   // native command. A corner can hide the third square behind a facade.
   const visible=new Set(frame.tiles.filter(t=>t.passable===true).map(t=>key(t.position)));
   let count=Math.min(3,route.length);
   while(count>1&&!visible.has(key(route[count-1])))count--;
   assert(visible.has(key(route[count-1])),"Next route square is not currently observed");
   await move(route.splice(0,count).at(-1));
  }
 };
 const mark=async name=>{stage=name;await canvas.screenshot({path:`${config.output}/${config.engine}-${name}.png`});captures.push(name);};
 await page.goto(config.origin+'/');await wait(()=>document.body.dataset.playReady==='true');
 await page.locator('#username').fill(config.username);await page.locator('#password').fill(config.password);
 await page.getByRole('button',{name:'Sign in',exact:true}).click();await wait(()=>document.body.dataset.phase==='selecting');
 await page.getByRole('button',{name:'Enter world',exact:true}).click();await ready();
 assert.equal(here().level,'arrival');const initial=here();
 assert.equal(await canvas.getAttribute('data-presentation'),'pixel-art');
 for(const selector of ['header','.controls','#gameplay','#position'])assert.equal(await page.locator(selector).isVisible(),false);
 await mark('frontage');
 assert.deepEqual(JSON.parse(await canvas.getAttribute('data-pixel-projection')).step,{x:48,y:32});
 await page.setViewportSize({width:900,height:1200});
 const count=commands.length;await click({x:12,y:9});await wait(()=>document.querySelector('#world-canvas').dataset.walkState==='draft');
 assert.deepEqual(JSON.parse(await canvas.getAttribute('data-walk-route')).at(-1),{i:12,j:9});assert.equal(commands.length,count);
 await mark('compact-draft');await page.keyboard.press('Escape');
 await page.setViewportSize({width:1280,height:800});
 const visited=[];
 for(const room of ['temple','trainers','outfitter','market','forge','lodge','bank']) {
  stage=`walking-to-${room}`;
  const enter=geography.connectivity.edges.find(e=>e.from_member==='arrival'&&e.to_member===room);
  const back=geography.connectivity.edges.find(e=>e.from_member===room&&e.to_member==='arrival');
  await walkTo(enter.from);assert.equal(here().level,room);visited.push(room);await mark(`inside-${room}`);
  await walkTo(back.from);assert.equal(here().level,'arrival');assert.deepEqual(here().position,back.to);
  await mark(`outside-${room}`);
 }
 await walkTo({x:17,y:9});await mark('tree-front');
 await walkTo({x:18,y:6});await mark('tree-behind');
 const treeBack=here();
 await page.getByRole('button',{name:'Reconnect',exact:true}).click();await ready();assert.deepEqual(here(),treeBack);
 const arrival=geography.members.find(m=>m.member==='arrival').landmarks.find(l=>l.role==='arrival').at;
 await walkTo(arrival);await mark('dock');
 const integerChecks=[];
 for(const size of [{width:1280,height:800,scale:2},{width:1920,height:1080,scale:3}]) {
  await page.setViewportSize(size);
  await wait(scale=>JSON.parse(document.querySelector('#world-canvas').dataset.pixelViewport).scale===scale,size.scale);
  const exclusions=frame.actors.map(a=>a.position.position);
  const check=await canvas.evaluate((canvas,{scale,exclusions})=>{
   const {width,height}=canvas,copy=document.createElement('canvas');copy.width=width;copy.height=height;
   const c=copy.getContext('2d');c.drawImage(canvas,0,0);const rgba=c.getImageData(0,0,width,height).data;
   const projection=JSON.parse(canvas.dataset.pixelProjection);
   let mismatches=0;const samples=[];
   for(let y=0;y<height;y+=scale)for(let x=0;x<width;x+=scale) {
    if(exclusions.some(p=>Math.abs(x/scale-projection.origin.x-p.x*projection.step.x)<55 && y/scale>projection.origin.y+p.y*projection.step.y-115 && y/scale<projection.origin.y+p.y*projection.step.y+20))continue;
    for(let dy=0;dy<scale;dy++)for(let dx=0;dx<scale;dx++)for(let k=0;k<4;k++)
      if(rgba[(y*width+x)*4+k]!==rgba[((y+dy)*width+x+dx)*4+k]){mismatches++;if(samples.length<20)samples.push({x,y,dx,dy,k,a:rgba[(y*width+x)*4+k],b:rgba[((y+dy)*width+x+dx)*4+k]});}
   }
   return {width,height,scale,mismatches,samples};
  },{scale:size.scale,exclusions});
  await writeFile(`${config.output}/${config.engine}-sampling-${size.scale}.json`,JSON.stringify(check,null,2));
  assert.equal(check.mismatches,0);integerChecks.push(check);await mark(`dock-${size.width}`);
 }
 await page.setViewportSize({width:1280,height:800});
 await walkTo(initial.position);assert.deepEqual(here(),initial);await mark('restored');
 assert.equal(await canvas.getAttribute('data-pixel-effects'),'webgl');
 const drawP95Ms=Number(await canvas.getAttribute('data-pixel-draw-p95-ms'));
 await page.getByRole('button',{name:'Sign out',exact:true}).click();await wait(()=>document.body.dataset.phase==='signed_out');
 assert.equal(await canvas.getAttribute('data-study-actor-count'),'0');assert.deepEqual(errors,[]);
 // Only the art mount is intercepted; live authority is never synthesized.
 await page.route('**/feel-assets/town-temple-tree.png',route=>route.fulfill({status:404,body:'missing'}));
 await page.reload();await wait(()=>document.body.dataset.playReady==='failed');
 assert(await page.locator('#login').isDisabled());
 await writeFile(`${config.output}/${config.engine}-pixel-exterior.json`,JSON.stringify({verdict:'PASS',engine:config.engine,
  commands:commands.length,accepted:results.filter(r=>r.disposition.kind==='accepted').length,captures,
  visited,integerChecks,shader_pipeline:true,native_double_click:true,compact_pointing:true,temple_entry_return:true,tree_front_behind_walk:true,
  reconnect_restores:true,missing_exterior_art_refused:true,drawP95Ms},null,2)+'\n');
}catch(error){
 await page?.screenshot({path:`${config.output}/${config.engine}-exterior-failure.png`}).catch(()=>{});
 await writeFile(`${config.output}/${config.engine}-exterior-failure.json`,JSON.stringify({stage,error:String(error),errors,position:frame?.observation_center},null,2));throw error;
}finally{await launched.stop();}
