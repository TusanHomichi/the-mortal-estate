import { describe, expect, it, vi } from "vitest";
import { PathControls, type WalkPresentation } from "../src/play/pathControls";
import { proposePath, routeDirections } from "../src/play/pathPlan";
import type { ControlView } from "../src/play/control";
import type { PathPreview } from "../src/play/pathPreview";
import type { GameplayFields } from "../src/authoritative/gameplay";
import type { Frame } from "../src/authoritative/state";

const frame = (): Frame => ({ observer_actor_id:"player", observation_center:{realm:"land",level:"town",position:{x:0,y:0}},
  logical_time:"0",ready_at:"0",can_act:true, actors:[],corpses:[],ground_items:[],gold_piles:[],
  tiles:Array.from({length:25},(_,i)=>({position:{x:i%5,y:Math.floor(i/5)},passable:true})) });
const state = (): ControlView => ({phase:"playing",busy:false,pending:false,characters:[],feedback:"",nextSequence:"1",creationOptions:[],createdCharacterId:null,
  snapshot:{generation:1,raw:"",envelope:{kind:"state_update",server_sequence:"1",world_revision:"1",static_scene_context:null,
    frame:frame() as Frame & GameplayFields}}});
const tick = async () => { await Promise.resolve(); await Promise.resolve(); };
function setup() {
  const requests: {path:readonly string[];resolve:(result:PathPreview|null)=>void}[]=[];
  const transport = { previewPath:(path:readonly string[])=>new Promise<PathPreview|null>(resolve=>requests.push({path,resolve})),
    cancelPathPreview:vi.fn(),command:vi.fn(()=>true) };
  let shown:WalkPresentation;
  const controls=new PathControls(transport,view=>{shown=view;});
  const current=state();controls.present(current);
  const accept=(i:number,count=requests[i]!.path.length)=>requests[i]!.resolve({ accepted_steps:String(count),
    steps:Array.from({length:count},()=>({outcome:{kind:"moved"}})) } as PathPreview);
  return {controls,transport,requests,current,accept,shown:()=>shown!};
}
describe("authoritative two-click path controls",()=>{
  it("first click draws feet without a command; endpoint confirmation submits exactly one path",async()=>{
    const s=setup();s.controls.click({x:3,y:0});
    expect(s.shown().kind).toBe("draft");expect(s.shown().route).toHaveLength(4);
    expect(s.transport.command).not.toHaveBeenCalled();s.accept(0);await tick();
    s.controls.click({x:3,y:0});expect(s.transport.command).not.toHaveBeenCalled();
    s.accept(1);await tick();
    expect(s.transport.command).toHaveBeenCalledExactlyOnceWith({kind:"move_path",path:["east","east","east"]});
    expect(s.shown().kind).toBe("committed");
  });
  it("a fast double-click waits for assessment and never sends duplicate commands",async()=>{
    const s=setup();s.controls.click({x:2,y:0});s.controls.click({x:2,y:0});s.controls.click({x:2,y:0});
    expect(s.requests).toHaveLength(1);s.accept(0);await tick();expect(s.transport.command).toHaveBeenCalledTimes(1);
  });
  it("cancellation or a replacement target prevents stale previews from committing",async()=>{
    const s=setup();s.controls.click({x:2,y:0});s.controls.click({x:2,y:0});s.controls.cancel();
    s.accept(0);await tick();expect(s.transport.command).not.toHaveBeenCalled();expect(s.shown().route).toBeNull();
    s.controls.click({x:2,y:0});s.controls.click({x:0,y:2});s.accept(1);await tick();
    expect(s.shown().route?.at(-1)).toEqual({i:0,j:2});expect(s.transport.command).not.toHaveBeenCalled();
  });
  it.each(["cooldown","disconnect","position"])("clears drafts on %s and ignores delayed replies",async reason=>{
    const s=setup();s.controls.click({x:2,y:0});s.controls.click({x:2,y:0});
    if(reason==="cooldown")s.current.snapshot!.envelope.frame.can_act=false;
    if(reason==="disconnect"){s.current.phase="disconnected";s.current.snapshot=null;}
    if(reason==="position")s.current.snapshot!.envelope.frame.observation_center.position={x:1,y:0};
    s.controls.present(s.current);s.accept(0);await tick();expect(s.transport.command).not.toHaveBeenCalled();
    expect(s.shown().route).toBeNull();
  });
  it("refuses partial assessments, locks committed input, and releases only on authority",async()=>{
    const s=setup();s.controls.click({x:3,y:0});s.controls.click({x:3,y:0});s.accept(0,1);await tick();
    expect(s.transport.command).not.toHaveBeenCalled();expect(s.shown().cursor).toBe("refused");
    s.controls.click({x:1,y:0});s.controls.click({x:1,y:0});s.accept(1);await tick();
    s.current.pending=true;s.controls.present(s.current);s.controls.cancel();s.controls.click({x:0,y:1});
    expect(s.shown().kind).toBe("committed");expect(s.shown().cursor).toBe("waiting");
    s.current.pending=false;s.current.snapshot!.envelope.frame.can_act=false;s.controls.present(s.current);
    expect(s.shown().cursor).toBe("waiting");
    s.current.snapshot!.envelope.frame.can_act=true;s.controls.present(s.current);expect(s.shown().route).toBeNull();
  });
  it("retains a draft across unrelated actor updates",()=>{
    const s=setup();s.controls.click({x:2,y:0});s.current.snapshot = { ...s.current.snapshot!, generation:2 };
    s.controls.present(s.current);expect(s.shown().route).toHaveLength(3);
  });
  it("does not paint a committed exterior route inside a portal destination",async()=>{
    const s=setup();s.controls.click({x:1,y:0});s.controls.click({x:1,y:0});s.accept(0);await tick();
    s.current.snapshot!.envelope.frame.observation_center.level="temple";
    s.current.snapshot!.envelope.frame.can_act=false;s.controls.present(s.current);
    expect(s.shown().route).toBeNull();expect(s.shown().hover).toBeNull();
  });
});
describe("observed route proposals",()=>{
  it("uses the previous shortest-step tie breaking, diagonals and three-step allowance",()=>{
    expect(routeDirections(proposePath(frame(),{x:3,y:3})!)).toEqual(["southeast","southeast","southeast"]);
    expect(proposePath(frame(),{x:4,y:0})).toBeNull();
  });
  it("does not invent unseen ground, cut blocked corners, or cross a portal en route",()=>{
    const f=frame();f.tiles=f.tiles.filter(t=>t.position.x!==1 || t.position.y!==0);
    expect(proposePath(f,{x:1,y:0})).toBeNull();
    expect(proposePath(f,{x:1,y:1})?.length).toBe(3);
    f.tiles=f.tiles.filter(t=>t.position.y===0);f.tiles.push({position:{x:1,y:0},passable:true,transition:{}});
    expect(proposePath(f,{x:2,y:0})).toBeNull();expect(proposePath(f,{x:1,y:0})).toHaveLength(2);
  });
});
