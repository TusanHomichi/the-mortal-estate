/** Wait for a presented frame, independently of authoritative action readiness. */
export async function waitForPixelPointing(page) {
 await page.waitForFunction(()=>{
  const c=document.querySelector('#world-canvas'),v=JSON.parse(c?.dataset.pixelViewport??'null');
  return v&&!!c.dataset.pixelProjection&&c.width===v.width*v.scale&&c.height===v.height*v.scale&&
    c.width<=innerWidth&&innerWidth-c.width<v.scale&&c.height<=innerHeight&&innerHeight-c.height<v.scale;
 },undefined,{timeout:45000});
}
export async function pixelCellPoint(page,cell,offsetY=0) {
 const canvas=page.locator('#world-canvas');await canvas.scrollIntoViewIfNeeded();await waitForPixelPointing(page);
 return canvas.evaluate((node,{cell,offsetY})=>{
  const p=JSON.parse(node.dataset.pixelProjection),v=JSON.parse(node.dataset.pixelViewport),box=node.getBoundingClientRect();
  return {x:box.left+(p.origin.x+cell.x*p.step.x)*box.width/v.width,
    y:box.top+(p.origin.y+cell.y*p.step.y+offsetY)*box.height/v.height};
 },{cell,offsetY});
}
