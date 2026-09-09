// Synthetic presentation fixture, never a game entry point or a wire producer.
import {PixelRenderer} from '../src/play/pixelRenderer.ts';
import {loadPixelPacket} from '../src/play/pixelPacket.ts';
import geography from '../../content/lands/first-expedition/generated/workbench_projection.json';
import promotion from '../../content/lands/first-expedition/promotion.json';

const packet=await loadPixelPacket();
const figures=['traveler','tomas','maude'];
// Explicit benchmark aliases ensure ten detailed figures, not unmapped NPC dots.
const fixture={...packet,manifest:{...packet.manifest,actor_figures:{...packet.manifest.actor_figures}}};
for(let i=0;i<10;i++)fixture.manifest.actor_figures[`crowd-${i}`]=figures[i%figures.length];
const canvas=document.querySelector('#world-canvas');
const renderer=new PixelRenderer(canvas,512,512,fixture);
let timer,updates=0,setup,phase=0;
function present() {
  const {level,count,positions,member,bounds}=setup;
  const actors=positions.slice(0,count).map((at,i)=>({actor_id:`crowd-${i}`,name:`Crowd ${i+1}`,
    position:{realm:'first_expedition',level,position:{x:at.x,y:at.y+(phase ? (i%2 ? -1 : 1) : 0)}}}));
  const tiles=member.cells.filter(c=>c.x>=bounds.min.x&&c.x<=bounds.max.x&&c.y>=bounds.min.y&&c.y<=bounds.max.y)
    .map(c=>({position:{x:c.x,y:c.y},passable:c.passable}));
  const frame={logical_time:'0',ready_at:'2500',can_act:true,observer_actor_id:'crowd-0',
    observation_center:actors[0].position,actors,tiles,corpses:[],ground_items:[],gold_piles:[]};
  renderer.present({generation:++updates,envelope:{frame,static_scene_context:{
    visual_manifest_digest:promotion.master.sha256,site:{realm:'first_expedition',level},bounds,
    walkable_mask:tiles.filter(c=>c.passable).map(c=>c.position)}},raw:''});
}
window.crowdFixture={
  start(level,count) {
    clearInterval(timer);renderer.clear();phase=0;updates=0;
    const positions=level==='temple'
      ? [{x:3,y:4},{x:1,y:3},{x:2,y:2},{x:3,y:3},{x:4,y:2},{x:5,y:3},{x:1,y:5},{x:2,y:6},{x:4,y:5},{x:5,y:6}]
      : [{x:13,y:12},{x:10,y:11},{x:11,y:10},{x:12,y:11},{x:14,y:10},{x:15,y:11},{x:10,y:13},{x:11,y:14},{x:14,y:13},{x:15,y:14}];
    const member=geography.members.find(m=>m.member===level);
    const bounds=level==='temple' ? {min:{x:0,y:0},max:{x:6,y:7}} : {min:{x:9,y:8},max:{x:17,y:16}};
    for(const [i,at] of positions.entries())for(const y of [at.y,at.y+(i%2 ? -1 : 1)]) {
      if(!member.cells.some(c=>c.passable&&c.x===at.x&&c.y===y))throw Error('Crowd route leaves authored floor');
    }
    setup={level,count,positions,member,bounds};present();
    phase=1;present();
    // Alternate before interpolation finishes, keeping every figure in motion.
    // This controls fixture input only; no gameplay clock or server is changed.
    timer=setInterval(()=>{phase=1-phase;present();},400);
  },
  state(){return {updates,actors:renderer.sprites.size,moving:[...renderer.sprites.values()].filter(s=>s.route).length,
    detailed:[...renderer.sprites.values()].filter(s=>s.figure).length,
    bounds:JSON.parse(canvas.dataset.pixelActorBounds),viewport:JSON.parse(canvas.dataset.pixelViewport)};},
  stop(){clearInterval(timer);renderer.dispose();},
};
