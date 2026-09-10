import assert from "node:assert/strict";
import {worldCellPoint,waitForWorldPointing} from "./world-pointing.mjs";
import { readFile, writeFile } from "node:fs/promises";
import { launchProofBrowser, PROOF_ENGINES } from "./serve.mjs";

let input=""; for await(const chunk of process.stdin) input+=chunk;
const config=JSON.parse(input); input="";
const engine=PROOF_ENGINES[config.engine]; if(!engine) throw Error("Unrostered engine");
const launched=await launchProofBrowser({name:config.engine,engine,executablePath:engine.executablePath(),trustedAuthority:config.authority});
const geography=JSON.parse(await readFile(new URL("../../content/lands/first-expedition/generated/workbench_projection.json",import.meta.url),"utf8"));
let page,frame,stage="starting";
const errors=[],commands=[],results=[],captures=[];
try {
  const context=launched.context || await launched.browser.newContext({viewport:{width:1400,height:1200}});
  page=await context.newPage(); await page.setViewportSize({width:1400,height:1200});
  page.on("pageerror",error=>errors.push(error.message));
  page.on("websocket",socket=>{
    socket.on("framesent",event=>{const e=JSON.parse(String(event.payload));if(e.kind==="command")commands.push(e);});
    socket.on("framereceived",event=>{const e=JSON.parse(String(event.payload));if(e.frame)frame=e.frame;if(e.kind==="command_result")results.push(e);});
  });
  const wait=(fn,arg)=>page.waitForFunction(fn,arg,{polling:30,timeout:45000});
  const canvas=page.locator("#world-canvas");
  const ready=()=>wait(()=>document.body.dataset.phase==="playing"&&document.querySelector("#world-canvas").dataset.canAct==="true"&&document.querySelector("#world-canvas").dataset.pending==="false");
  const here=()=>structuredClone(frame.observation_center);
  async function mark(name){stage=name;await canvas.screenshot({path:`${config.output}/${config.engine}-${name}.png`});captures.push(name);}
  const point=(cell,offsetY=0)=>worldCellPoint(page,cell,offsetY);
  async function click(cell,options){const p=await point(cell);await page.mouse.click(p.x,p.y,options);}
  async function committed(count){
    await wait(()=>document.querySelector("#world-canvas").dataset.canAct==="false");
    assert.equal(commands.length,count+1);
    await ready();assert.equal(commands.length,count+1);assert.equal(results.at(-1)?.disposition.kind,"accepted");
  }
  async function move(cell){await ready();const count=commands.length;await click(cell);await wait(()=>document.querySelector("#world-canvas").dataset.walkState==="draft");await click(cell);await committed(count);}
  async function walkTo(target){
    const start=here(),key=p=>`${p.x}:${p.y}`;
    const member=geography.members.find(m=>m.member===start.level),open=new Set(member.cells.filter(c=>c.passable).map(key));
    for(const door of member.doors??[])if(door.hidden)open.delete(key(door.at));
    for(const e of geography.connectivity.edges.filter(e=>e.from_member===start.level&&e.direction==="passage"))if(key(e.from)!==key(target))open.delete(key(e.from));
    const queue=[start.position],previous=new Map([[key(start.position),null]]);
    for(let index=0;index<queue.length;index++){
      const p=queue[index];if(key(p)===key(target))break;
      for(const [dx,dy] of [[0,-1],[1,0],[0,1],[-1,0]]){const q={x:p.x+dx,y:p.y+dy};if(open.has(key(q))&&!previous.has(key(q))){previous.set(key(q),p);queue.push(q);}}
    }
    assert(previous.has(key(target)),"No authored route");const route=[];
    for(let p=target;previous.get(key(p))!==null;p=previous.get(key(p)))route.unshift(p);
    while(route.length){
      // Around corners the next three authored cells need not all be visible.
      // The real UI is allowed to propose only the currently observed prefix.
      let length=0;
      for(const cell of route.slice(0,3)){
        if(!frame.tiles.some(t=>t.terrain_id&&key(t.position)===key(cell)))break;
        length++;
        const tile=frame.tiles.find(t=>key(t.position)===key(cell));
        if(tile.transition?.navigation==='door'&&tile.transition.door_open!==true)break;
      }
      assert(length>0,"Next authored route cell must be observed from its neighbor");
      await move(route.splice(0,length).at(-1));
    }
  }
  async function openActor(id){
    // Click the presented body, including interpolation and shared-square
    // separation. A patrol need not stop at its latest authoritative endpoint.
    const actor=frame.actors.find(a=>a.actor_id===id);assert(actor);
    await waitForWorldPointing(page);
    const p=await canvas.evaluate((node,id)=>{
      if(node.dataset.presentation==='dungeon-3d'){
        const hit=JSON.parse(node.dataset.dungeonActorPoints).find(hit=>hit.id===id);if(!hit)throw Error('Actor not drawn');
        const box=node.getBoundingClientRect();return {x:box.left+hit.px*box.width,y:box.top+hit.py*box.height};
      }
      const hit=JSON.parse(node.dataset.pixelActorBounds).find(hit=>hit.id===id);
      if(!hit)throw Error('Resident is not presented');
      const box=node.getBoundingClientRect(),viewport=JSON.parse(node.dataset.pixelViewport);
      return {x:box.left+(hit.x+hit.width*.5)*box.width/viewport.width,y:box.top+(hit.y+hit.height*.5)*box.height/viewport.height};
    },id);
    await page.mouse.click(p.x,p.y,{button:"right"});
    await page.locator(".resident-dialog[open]").waitFor();
    assert.equal(await page.locator("#resident-title").textContent(),actor.name);
  }
  async function traverse(direction){
    await ready();
    const chosen=frame.action_options.filter(a=>a.enabled&&a.intent?.kind==="traverse");
    assert.equal(chosen.length,1);assert.equal(chosen[0].intent.traversal,direction);
    const count=commands.length;
    await click(here().position,{clickCount:2,delay:60});await committed(count);
  }
  for(const entry of ["/", "/index.html", "/index.html?study=diagnostic", "/play.html?study=first-expedition"]) {
    await page.goto(config.origin+entry);
    await wait(()=>document.body.dataset.playReady==="true");
    assert.equal(await canvas.getAttribute("data-presentation"),"pixel-art",`pixel entry must be stable: ${entry}`);
  }
  await wait(()=>document.body.dataset.playReady==="true");
  await page.locator("#username").fill(config.username);await page.locator("#password").fill(config.password);
  await page.getByRole("button",{name:"Sign in",exact:true}).click();await wait(()=>document.body.dataset.phase==="selecting");
  await page.getByRole("button",{name:"Enter world",exact:true}).click();await ready();
  assert.equal(await canvas.getAttribute("data-presentation"),"pixel-art","build cannot switch presentation through URL");
  assert.equal(await canvas.getAttribute("data-presentation-error"),null);
  assert.equal(await canvas.getAttribute("data-pixel-native-size"),"700x600");
  assert.equal(await canvas.evaluate(node=>node.width),1400);
  for(const selector of [".controls","#gameplay","#position",".legend","#settings","header"]) {
    assert.equal(await page.locator(selector).isVisible(),false,`temporary HUD must be absent: ${selector}`);
  }
  await page.screenshot({path:`${config.output}/${config.engine}-minimal-playfield.png`});
  const idleCommands=commands.length;
  await click(here().position,{clickCount:2,delay:60});
  assert.equal(commands.length,idleCommands,"double-clicking an ordinary occupied square cannot invent an action");
  const desktopWidth=(await canvas.boundingBox()).width;
  assert(desktopWidth>=1000,"a wide desktop must give the room its available detail budget");
  await page.setViewportSize({width:900,height:1200});
  await page.waitForFunction(()=>{const c=document.querySelector('#world-canvas');return c.width===900&&!!c.dataset.pixelProjection;});
  const compactWidth=(await canvas.boundingBox()).width;
  assert(compactWidth<=900&&compactWidth<desktopWidth,"the room must still fit a compact window");
  const compactCommands=commands.length;
  await click({x:3,y:3});await wait(()=>document.querySelector("#world-canvas").dataset.walkState==="draft");
  assert.equal(commands.length,compactCommands,"resizing cannot submit movement");
  assert.deepEqual(JSON.parse(await canvas.getAttribute("data-walk-route")).at(-1),{i:3,j:3},"compact pointing must retain shared cell identity");
  await page.keyboard.press("Escape");
  await page.setViewportSize({width:1400,height:1200});
  await page.waitForFunction(()=>{const c=document.querySelector('#world-canvas');return c.width===1400&&!!c.dataset.pixelProjection;});
  assert.equal(here().level,"temple");await mark("temple-arrival");
  // A known clear floor edge must come from the overlay, not the room PNG.
  const grid=await canvas.evaluate(async node=>{
    const image=new Image();image.src="/feel-assets/temple.png";await image.decode();
    const plate=document.createElement("canvas");plate.width=512;plate.height=512;
    const ctx=plate.getContext("2d");ctx.drawImage(image,0,0);
    const viewport=JSON.parse(node.dataset.pixelViewport),projection=JSON.parse(node.dataset.pixelProjection);
    const offset={x:projection.origin.x-76,y:projection.origin.y-110};
    const read=(context,x,y)=>Array.from(context.getImageData(x,y,1,1).data);
    const copy=document.createElement('canvas');copy.width=node.width;copy.height=node.height;
    const display=copy.getContext('2d');display.drawImage(node,0,0);
    const rendered=(x,y)=>read(display,(x+offset.x)*viewport.scale,(y+offset.y)*viewport.scale);
    return {plate:read(ctx,296,330),grid:rendered(296,330),plainPlate:read(ctx,298,330),plain:rendered(298,330),
      bedPlate:read(ctx,450,176),bed:rendered(450,176),bedNearPlate:read(ctx,449,176),bedNear:rendered(449,176)};
  });
  assert(grid.grid.slice(0,3).every((value,index)=>value<grid.plate[index]-10),"persistent grid must be drawn over the plain room plate at the cell boundary");
  const luminance=rgb=>rgb.slice(0,3).reduce((sum,x)=>sum+x,0);
  assert(luminance(grid.grid)/luminance(grid.plate)<luminance(grid.plain)/luminance(grid.plainPlate)-.08,"grid darkening must exceed the local light field");
  assert(Math.abs(luminance(grid.bed)/luminance(grid.bedPlate)-luminance(grid.bedNear)/luminance(grid.bedNearPlate))<.08,"bed lighting must remain continuous over the occluded grid boundary");
  const before=here(),count=commands.length;
  await click({x:3,y:3});await wait(()=>document.querySelector("#world-canvas").dataset.walkState==="draft");
  assert.equal(commands.length,count);assert.deepEqual(here(),before);await mark("temple-footprints");
  await page.keyboard.press("Escape");assert.equal(await canvas.getAttribute("data-walk-state"),"idle");
  await click({x:3,y:3});await wait(()=>document.querySelector("#world-canvas").dataset.walkState==="draft");
  await click({x:3,y:3});await wait(()=>document.querySelector("#world-canvas").dataset.canAct==="false");
  assert.deepEqual(JSON.parse(await canvas.getAttribute("data-pixel-motion-route")),
    [{x:3,y:6},{x:3,y:5},{x:3,y:4},{x:3,y:3}],"receipt/frame handoff must retain every traveled square");
  assert(Number(await canvas.getAttribute("data-pixel-motion-duration-ms"))>=1500,"three squares must not collapse into a single short glide");
  await click({x:3,y:6});assert.equal(commands.length,count+1);await ready();assert.deepEqual(here().position,{x:3,y:3});
  await openActor("tomas");await mark("tomas-menu");await page.locator(".resident-dialog").getByRole("button",{name:"Close",exact:true}).click();
  for(let tries=0;tries<5&&!frame.services_here.some(s=>s.actor_id==="tomas");tries++)await walkTo(frame.actors.find(a=>a.actor_id==="tomas").position.position);
  assert(frame.services_here.some(s=>s.actor_id==="tomas"),"Tomas's moving service must resolve at his current square");
  await openActor("tomas");assert(await page.locator(".resident-dialog div button").count()>0);await mark("tomas-services");
  await page.locator(".resident-dialog").getByRole("button",{name:"Close",exact:true}).click();
  await walkTo(frame.actors.find(a=>a.actor_id==="balm_seller").position.position);await openActor("balm_seller");
  const purchase=page.locator(".resident-dialog").getByRole("button",{name:/^Buy .*balm/i}).first();assert(await purchase.isEnabled());
  const purchases=commands.length;await purchase.click();await committed(purchases);
  assert(frame.carried.items.some(i=>i.item.item_definition_id==="healing_balm"));await mark("maude-purchase");
  await page.locator(".resident-dialog").getByRole("button",{name:"Close",exact:true}).click();
  await walkTo({x:0,y:4});await traverse("stairs_down");
  assert.equal(here().level,"d1_entry");
  assert.deepEqual(here().position,{x:24,y:7});
  const dungeon=geography.members.find(member=>member.member==="d1_entry");
  assert.deepEqual([dungeon.width,dungeon.height],[36,43]);await mark("descent");
  const scenery=JSON.parse(await canvas.getAttribute('data-dungeon-view')).walls;
  assert.equal(await canvas.getAttribute('data-presentation'),'dungeon-3d');
  assert(scenery.some(p=>!p.door&&p.height>2));
  assert(scenery.some(p=>p.door&&p.axis==='x'));
  assert(scenery.some(p=>p.door&&p.axis==='y'));
  await walkTo({x:23,y:7});
  const doorway=()=>frame.tiles.find(tile=>tile.position.x===23&&tile.position.y===6)?.transition;
  assert.equal(doorway()?.navigation,"door");assert.equal(doorway()?.door_open,false);
  await move({x:23,y:6});assert.deepEqual(here().position,{x:23,y:6});
  assert.equal(doorway()?.door_open,true);
  await walkTo({x:21,y:3});await mark("dungeon-north-room");
  await walkTo({x:23,y:7});assert.equal(doorway()?.door_open,true);
  await walkTo({x:23,y:9});await ready();
  const beforeMenu=commands.length;
  await openActor("cellar_scavenger");
  assert.equal(commands.length,beforeMenu,"opening a creature must not attack it");
  const attack=frame.action_options.find(a=>a.enabled&&a.intent?.kind==="physical_attack"&&
    a.intent.target_actor_id==="cellar_scavenger"&&a.intent.mode==="fight");
  assert(attack,"the real authority must offer a nearby weapon attack");
  const expectedAttack=structuredClone(attack.intent);
  await mark("creature-actions");
  await page.locator(".resident-dialog").locator(`button[data-action=${JSON.stringify(`character/${attack.id}`)}]`).click();
  await committed(beforeMenu);
  assert.deepEqual(commands.at(-1).intent,expectedAttack,"the UI must submit the exact offered target and authorization");
  const dialog=page.locator(".resident-dialog[open]");
  if(await dialog.count())await dialog.getByRole("button",{name:"Close",exact:true}).click();
  // Traverse real authored routes through all four floors, then retrace them.
  const descend=async(level,at,landing)=>{
    await walkTo(at);await traverse("stairs_down");assert.equal(here().level,level);
    assert.deepEqual(here().position,landing);await mark(`${level}-arrival`);
  };
  await descend("d2",{x:15,y:3},{x:14,y:3});
  await descend("d3",{x:4,y:3},{x:1,y:3});
  await descend("d4",{x:9,y:17},{x:7,y:17});
  // Stand on the return stair and use the actor menu's exact server offer.
  await walkTo({x:8,y:17});
  await openActor(frame.observer_actor_id);
  const offeredStair=frame.action_options.find(a=>a.enabled&&a.intent?.kind==="traverse");
  assert.equal(offeredStair?.intent.traversal,"stairs_up");
  const stairsCount=commands.length,expectedStair=structuredClone(offeredStair.intent);
  await mark("underfoot-stair-menu");
  await page.locator(".resident-dialog").locator(`button[data-action=${JSON.stringify(`character/${offeredStair.id}`)}]`).click();
  await committed(stairsCount);assert.deepEqual(commands.at(-1).intent,expectedStair);
  assert.equal(here().level,"d3");
  if(await page.locator(".resident-dialog[open]").count())await page.locator(".resident-dialog").getByRole("button",{name:"Close",exact:true}).click();
  await walkTo({x:2,y:3});await traverse("stairs_up");assert.equal(here().level,"d2");
  await walkTo({x:15,y:3});await traverse("stairs_up");assert.equal(here().level,"d1_entry");
  await walkTo({x:25,y:7});await traverse("stairs_up");
  assert.equal(here().level,"temple");await walkTo({x:3,y:5});
  const doubleClickCount=commands.length;
  await click({x:3,y:6},{clickCount:2,delay:60});await committed(doubleClickCount);
  assert.deepEqual(here().position,{x:3,y:6});await mark("temple-return");
  const position=here();await page.getByRole("button",{name:"Reconnect",exact:true}).click();await ready();assert.deepEqual(here(),position);
  assert.equal(await canvas.getAttribute("data-walk-state"),"idle");assert.equal(await canvas.getAttribute("data-presentation-error"),null);
  const drawP95Ms=Number(await canvas.getAttribute("data-pixel-draw-p95-ms"));
  const resources=await page.evaluate(()=>performance.getEntriesByType("resource").map(r=>r.name));
  assert(resources.some(url=>url.includes("dungeon-adventurer.glb")),"World build must load its bound dungeon body");
  await page.getByRole("button",{name:"Sign out",exact:true}).click();await wait(()=>document.body.dataset.phase==="signed_out");
  assert.equal(await canvas.getAttribute("data-study-actor-count"),"0");assert.deepEqual(errors,[]);
  assert.deepEqual(await canvas.evaluate(node=>{const copy=document.createElement('canvas');copy.width=node.width;copy.height=node.height;const c=copy.getContext('2d');c.drawImage(node,0,0);return Array.from(c.getImageData(256,256,1,1).data);}),[21,21,25,255]);
  // Mutate only art responses after logout; no gameplay transport is intercepted.
  await page.route("**/feel-assets/pixel-manifest.json",route=>route.fulfill({status:200,contentType:"application/json",body:"{}"}));
  await page.reload();await wait(()=>document.body.dataset.playReady==="failed");
  assert(await page.locator("#login").isDisabled(),"a mismatched digest must refuse startup");
  await page.unroute("**/feel-assets/pixel-manifest.json");
  await page.route("**/feel-assets/dungeon-adventurer.glb",route=>route.fulfill({status:404,body:"missing"}));
  await page.reload();await wait(()=>document.body.dataset.playReady==="failed");
  assert(await page.locator("#login").isDisabled(),"missing imagery must refuse startup");
  await writeFile(`${config.output}/${config.engine}-pixel-temple.json`,JSON.stringify({verdict:"PASS",engine:config.engine,
    rendering:launched.rendering,adapter:launched.renderer,native_viewport:{width:700,height:600,scale:2},desktop_width:desktopWidth,compact_width:compactWidth,
    temporary_hud_absent:true,direct_stair_double_click:true,responsive_pointing:true,draw_p95_ms:drawP95Ms,commands:commands.length,captures,
    initial_click_nonmutating:true,cooldown_locked:true,resident_identity:true,moving_service:true,purchase:true,descent_and_return:true,
    creature_menu_nonmutating:true,offered_physical_attack:true,attack_target_and_authorization_preserved:true,
    four_floor_round_trip:true,underfoot_stair_menu:true,offered_traversal_preserved:true,required_dungeon_art:true,
    raised_oriented_walls_and_doors:true,occluder_opacity:.4,
    reconnect:true,logout_clears:true,bound_3d_assets:true,build_mode_pinned:true,stale_digest_refused:true,missing_image_refused:true,
    grid_independent_of_art:true,grid_occluded_by_furniture:true,committed_route_preserved:true},null,2)+"\n");
}catch(error){
  await page?.screenshot({path:`${config.output}/${config.engine}-failure.png`}).catch(()=>{});
  await writeFile(`${config.output}/${config.engine}-failure.json`,JSON.stringify({stage,error:String(error),errors,position:frame?.observation_center,
    intents:commands.map(c=>c.intent),lastResult:results.at(-1),frame},null,2));throw error;
}finally{await launched.stop();}
