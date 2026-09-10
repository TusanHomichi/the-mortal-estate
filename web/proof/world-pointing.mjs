/** Wait for a presented frame, independently of authoritative action readiness. */
export async function waitForWorldPointing(page) {
 await page.waitForFunction(()=>{
  const c=document.querySelector('#world-canvas');
  if(c?.dataset.presentation==='dungeon-3d')return !!c.dataset.dungeonView&&!!c.dataset.dungeonPoints&&c.clientWidth===innerWidth&&c.clientHeight===innerHeight;
  const v=JSON.parse(c?.dataset.pixelViewport??'null');
  if(!v||!c.dataset.pixelProjection||c.width!==Math.round(v.width*v.scale)||c.height!==Math.round(v.height*v.scale))return false;
  return c.width<=innerWidth&&innerWidth-c.width<v.scale&&c.height<=innerHeight&&innerHeight-c.height<v.scale;
 },undefined,{timeout:45000});
}
export async function worldCellPoint(page,cell,offsetY=0) {
 const canvas=page.locator('#world-canvas');await canvas.scrollIntoViewIfNeeded();await waitForWorldPointing(page);
 return canvas.evaluate((node,{cell,offsetY})=>{
  if(node.dataset.presentation==='dungeon-3d'){
    const p=JSON.parse(node.dataset.dungeonPoints).find(p=>p.x===cell.x&&p.y===cell.y);
    if(!p)throw Error('Requested cell is outside the observed dungeon display.');
    const box=node.getBoundingClientRect();return {x:box.left+p.px*box.width,y:box.top+p.py*box.height+offsetY};
  }
  const p=JSON.parse(node.dataset.pixelProjection),v=JSON.parse(node.dataset.pixelViewport),box=node.getBoundingClientRect();
  return {x:box.left+(p.origin.x+cell.x*p.step.x)*box.width/v.width,
    y:box.top+(p.origin.y+cell.y*p.step.y+offsetY)*box.height/v.height};
 },{cell,offsetY});
}
