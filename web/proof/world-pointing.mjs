/** Wait for the actual 3D camera projection, independently of action readiness. */
export async function waitForWorldPointing(page) {
 await page.waitForFunction(()=>{
  const c=document.querySelector('#world-canvas');
  return c?.dataset.presentation==='world-3d'&&!!c.dataset.worldView&&!!c.dataset.worldPoints&&c.clientWidth===innerWidth&&c.clientHeight===innerHeight;
 },undefined,{timeout:45000});
}
export async function worldCellPoint(page,cell,offsetY=0) {
 const canvas=page.locator('#world-canvas');await canvas.scrollIntoViewIfNeeded();await waitForWorldPointing(page);
 return canvas.evaluate((node,{cell,offsetY})=>{
  const p=JSON.parse(node.dataset.worldPoints).find(p=>p.x===cell.x&&p.y===cell.y);
  if(!p)throw Error('Requested cell is outside the observed world display.');
  const box=node.getBoundingClientRect();return {x:box.left+p.px*box.width,y:box.top+p.py*box.height+offsetY};
 },{cell,offsetY});
}
