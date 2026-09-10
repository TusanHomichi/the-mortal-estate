import {describe,it,expect} from 'vitest';
import {combatCues,movementRoute,movementSeconds,type CombatCue} from '../src/play/dungeon/motion';

const MARTIAL='actor:fighter',RIVAL='actor:brawler';
const identity=(actor_id:string,kind='player')=>({actor_id,name:actor_id,kind});
const place=(x:number,y:number,level='d1_entry',realm='first_expedition')=>({realm,level,position:{x,y}});
type Outcome={kind:string;damage?:number;armor_reduction?:number;wound_before?:string;wound_after?:string;target_hp?:number;current_time?:string;ready_at?:string};
const HIT:Outcome={kind:'hit',damage:4,armor_reduction:0,wound_before:'unhurt',wound_after:'wounded',target_hp:9};
const ARMORED:Outcome={kind:'hit',damage:4,armor_reduction:3,wound_before:'unhurt',wound_after:'unhurt',target_hp:12};
const MISSED:Outcome={kind:'missed'},BLOCKED:Outcome={kind:'blocked'},NO_SIGHT:Outcome={kind:'no_sight'};
const NOT_READY:Outcome={kind:'not_ready',current_time:'4000',ready_at:'9007199254740993'};
const OUTCOMES:Outcome[]=[HIT,MISSED,BLOCKED,NO_SIGHT,NOT_READY];
const MODES=['fight','kick','jumpkick','poke','shoot','throw'];
const physical=(mode:string,outcome:Outcome,source:unknown,target:unknown):unknown=>({kind:'feedback',
  cue:{kind:'physical_combat',source,target,location:place(25,9),mode,outcome}});
const swing=(mode:string,outcome:Outcome=MISSED,source:unknown=identity(MARTIAL),target:unknown=identity(RIVAL,'monster')):unknown =>
  physical(mode,outcome,source,target);
const moved=(from:{x:number;y:number},to:{x:number;y:number},navigation:unknown='walk',actorId=MARTIAL,realm='first_expedition',fromLevel='d1_entry',toLevel='d1_entry'):unknown =>
  ({kind:'actor_moved',actor_id:actorId,from:{realm,level:fromLevel,position:from},to:{realm,level:toLevel,position:to},navigation});
const both=new Set([MARTIAL,RIVAL]);
function frozen<T>(value:T):T {if(value&&typeof value==='object')Object.values(value).forEach(frozen);Object.freeze(value);return value;}
const faces=(cues:CombatCue[])=>cues.map(cue=>[cue.clip,cue.faceActorId]);

describe('dungeon combat cue planning',()=>{
  it('reads attacker cues from real hit, missed, and blocked rows and ignores the rest',()=>{
    expect(combatCues([swing('fight',HIT)],MARTIAL,both,0,true)).toEqual([{actorId:MARTIAL,clip:'jab_left',faceActorId:RIVAL}]);
    for(const outcome of [MISSED,BLOCKED])expect(combatCues([swing('fight',outcome)],MARTIAL,both,0,true)).toHaveLength(1);
    for(const outcome of [NO_SIGHT,NOT_READY])expect(combatCues([swing('fight',outcome)],MARTIAL,both,0,true)).toEqual([]);
    expect(combatCues([physical('fight',NOT_READY,identity(MARTIAL),identity(MARTIAL))],MARTIAL,both,0,true)).toEqual([]);
  });
  it('rotates the four accepted unarmed punches deterministically from variation',()=>{
    const clip=(variation:number)=>combatCues([swing('fight')],MARTIAL,both,variation,true)[0]?.clip;
    expect([0,1,2,3,4,5].map(clip)).toEqual(['jab_left','jab_right','uppercut_right','hook_left','jab_left','jab_right']);
    expect([3.9,-1,-5,Number.NaN,Number.POSITIVE_INFINITY].map(clip)).toEqual(['hook_left','hook_left','hook_left','jab_left','jab_left']);
    const pair=[swing('fight'),swing('fight')];
    expect(combatCues(pair,MARTIAL,both,0,true).map(cue=>cue.clip)).toEqual(['jab_left','jab_right']);
    expect(combatCues(pair,MARTIAL,both,1,true).map(cue=>cue.clip)).toEqual(['jab_right','uppercut_right']);
    expect(combatCues([swing('kick'),swing('fight')],MARTIAL,both,0,true).map(cue=>cue.clip)).toEqual(['jab_left']);
  });
  it('gives the flying kick to a jumpkick whatever the armament or variation',()=>{
    expect(combatCues([swing('jumpkick',HIT)],MARTIAL,both,3,false)).toEqual([{actorId:MARTIAL,clip:'flying_kick',faceActorId:RIVAL}]);
    expect(combatCues([swing('jumpkick',BLOCKED)],MARTIAL,both,2,true).map(cue=>cue.clip)).toEqual(['flying_kick']);
  });
  it('leaves modes without an accepted attacker clip alone',()=>{
    for(const mode of ['kick','poke','shoot','throw'])expect(combatCues([swing(mode,HIT)],MARTIAL,both,0,true)).toEqual([]);
    expect(combatCues([swing('fight',HIT)],MARTIAL,both,0,false)).toEqual([]);
  });
  it('covers every wire mode and outcome for a martial source and a martial target',()=>{
    const attacker:{outcome:Outcome;mode:string;clip:string}[]=[];
    for(const mode of MODES)for(const outcome of OUTCOMES){
      // no_sight and not_ready mean the swing never happened: no mode animates from them.
      const swung=outcome.kind==='hit'||outcome.kind==='missed'||outcome.kind==='blocked';
      const clip=!swung?null:mode==='jumpkick'?'flying_kick':mode==='fight'?'jab_left':null;
      attacker.push({outcome,mode,clip:clip??''});
    }
    for(const {outcome,mode,clip} of attacker){
      const cues=combatCues([swing(mode,outcome)],MARTIAL,both,0,true);
      expect(cues).toEqual(clip?[{actorId:MARTIAL,clip,faceActorId:RIVAL}]:[]);
    }
    const blocks:Record<string,string>={fight:'block_high',kick:'block_side',jumpkick:'block_lean',poke:'block_cover'};
    for(const mode of MODES)for(const outcome of OUTCOMES){
      const source=identity(RIVAL,'monster');
      const cues=combatCues([physical(mode,outcome,source,identity(MARTIAL))],MARTIAL,both,0,true);
      const clip=outcome.kind==='blocked'?blocks[mode]:undefined;
      expect(cues).toEqual(clip?[{actorId:MARTIAL,clip,faceActorId:RIVAL}]:[]);
    }
  });
  it('keeps guard animation off armored hits and out of ranged modes',()=>{
    expect(combatCues([physical('fight',ARMORED,identity(RIVAL),identity(MARTIAL))],MARTIAL,both,0,true)).toEqual([]);
    for(const mode of ['shoot','throw'])expect(combatCues([physical(mode,BLOCKED,identity(RIVAL),identity(MARTIAL))],MARTIAL,both,0,true)).toEqual([]);
    expect(combatCues([physical('kick',BLOCKED,identity(RIVAL),identity(MARTIAL))],MARTIAL,both,0,true).map(cue=>cue.clip)).toEqual(['block_side']);
  });
  it('keeps hidden counterparts and unknown block sources facing null',()=>{
    const hidden=new Set([MARTIAL]);
    expect(combatCues([swing('fight',HIT)],MARTIAL,hidden,0,true)).toEqual([{actorId:MARTIAL,clip:'jab_left',faceActorId:null}]);
    expect(combatCues([physical('kick',BLOCKED,identity(RIVAL),identity(MARTIAL))],MARTIAL,hidden,0,true)).toEqual([{actorId:MARTIAL,clip:'block_side',faceActorId:null}]);
    expect(combatCues([physical('fight',BLOCKED,null,identity(MARTIAL))],MARTIAL,hidden,0,true)).toEqual([{actorId:MARTIAL,clip:'block_high',faceActorId:null}]);
    expect(combatCues([swing('fight',HIT)],MARTIAL,new Set([RIVAL]),0,true)).toEqual([]);
    for(const cue of combatCues([swing('fight',HIT),physical('poke',BLOCKED,identity(RIVAL),identity(MARTIAL))],MARTIAL,hidden,0,true))expect(cue.faceActorId).toBeNull();
  });
  it('ignores non-feedback rows, other cues, and malformed rows in event order',()=>{
    const noise:unknown[]=[{kind:'actor_moved',actor_id:MARTIAL,from:place(24,9),to:place(25,9),navigation:'walk'},
      {kind:'inspected',location:place(25,9),tile:'expedition_floor',nearby_actors:[],exits:[],ground_items:[],tile_move_cost:1},
      {kind:'feedback',cue:{kind:'skill_critique',service_id:'training_hall',track_id:'staff',track_display:'Staff',level:0,critique_rank:null,level_title:null}},
      {kind:'feedback',cue:{kind:'physical_combat',source:identity(MARTIAL),target:identity(RIVAL),location:null,mode:'fight'}},
      {kind:'feedback',cue:{kind:'physical_combat',source:identity(MARTIAL),target:null,location:null,mode:'fight',outcome:HIT}},
      {kind:'feedback'}];
    expect(combatCues(noise,MARTIAL,both,0,true)).toEqual([]);
    const ordered=[swing('jumpkick',HIT),swing('fight',HIT)];
    // The jumpkick does not consume a punch slot, so variation 2 still lands on index 2.
    expect(faces(combatCues(ordered,MARTIAL,both,2,true))).toEqual([['flying_kick',RIVAL],['uppercut_right',RIVAL]]);
  });
  it('does not mutate events or the visibility set',()=>{
    const events=frozen([swing('fight',HIT),physical('jumpkick',BLOCKED,identity(RIVAL),identity(MARTIAL)),swing('fight',MISSED)]);
    const cells=frozen(new Set([MARTIAL,RIVAL])),before=JSON.stringify(events);
    expect(combatCues(events,MARTIAL,cells,1,true).map(cue=>cue.clip)).toEqual(['jab_right','block_lean','uppercut_right']);
    expect(JSON.stringify(events)).toBe(before);
  });
});

describe('dungeon movement route extraction',()=>{
  const cells=()=>new Set(['24:9','25:9','25:10','26:10','26:9']);
  it('keeps a chained three-cell walk route that ends on the actor cell',()=>{
    const events=[moved({x:24,y:9},{x:25,y:9}),moved({x:25,y:9},{x:25,y:10})];
    expect(movementRoute(events,MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10}))
      .toEqual([{x:24,y:9},{x:25,y:9},{x:25,y:10}]);
  });
  it('accepts a local open door and a swim as flat local steps',()=>{
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9}),moved({x:25,y:9},{x:25,y:10},'door')],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10}))
      .toEqual([{x:24,y:9},{x:25,y:9},{x:25,y:10}]);
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9},'swim'),moved({x:25,y:9},{x:25,y:10},'swim')],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10}))
      .toEqual([{x:24,y:9},{x:25,y:9},{x:25,y:10}]);
  });
  it('refuses a chain with an incomplete or unfinished path',()=>{
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9}),moved({x:26,y:9},{x:25,y:10})],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9})],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9}),moved({x:25,y:9},{x:25,y:10})],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:9})).toBeNull();
    expect(movementRoute([],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:9})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9},'walk',RIVAL)],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:9})).toBeNull();
  });
  it('refuses level, realm, view, and non-flat traversals',()=>{
    const walk=[moved({x:24,y:9},{x:25,y:9})];
    for(const navigation of [{stairs:{direction:'down'}},{climb:{direction:'up'}},'pit','passage','portal'])
      expect(movementRoute([...walk,moved({x:25,y:9},{x:25,y:10},navigation)],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9},'walk',MARTIAL,'first_expedition','d1_entry','d2')],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:9})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9},'walk',MARTIAL,'first_expedition')],MARTIAL,'d1_entry','deep_woods',cells(),{x:25,y:9})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:27,y:9})],MARTIAL,'d1_entry','first_expedition',cells(),{x:27,y:9})).toBeNull();
    expect(movementRoute([moved({x:23,y:9},{x:24,y:9})],MARTIAL,'d1_entry','first_expedition',cells(),{x:24,y:9})).toBeNull();
    expect(movementRoute([moved({x:24,y:9},{x:25,y:9},'walk'),moved({x:25,y:9},{x:25,y:10},'stairs')],MARTIAL,'d1_entry','first_expedition',cells(),{x:25,y:10})).toBeNull();
  });
  it('does not mutate events or fabricate intermediate corners',()=>{
    const events=frozen([moved({x:24,y:9},{x:25,y:9}),moved({x:25,y:9},{x:25,y:10},'door'),moved({x:25,y:10},{x:26,y:10})]);
    const visible=frozen(cells()),before=JSON.stringify(events),route=movementRoute(events,MARTIAL,'d1_entry','first_expedition',visible,{x:26,y:10});
    expect(route).toEqual([{x:24,y:9},{x:25,y:9},{x:25,y:10},{x:26,y:10}]);
    expect(JSON.stringify(events)).toBe(before);
    expect(route).not.toBe(events);
  });
});

describe('dungeon movement clock',()=>{
  it('bounds the server interval rather than inventing one',()=>{
    expect(movementSeconds('1000','1500')).toBe(0.5);
    expect(movementSeconds('4000','4000')).toBe(0.18);
    expect(movementSeconds('4000','3900')).toBe(0.18);
    expect(movementSeconds('1000','1050')).toBe(0.05);
    expect(movementSeconds('1000','1079')).toBe(0.079);
    expect(movementSeconds('1000','1080')).toBeCloseTo(0.08);
    expect(movementSeconds('1000','4000')).toBe(3);
    expect(movementSeconds('1000','4001')).toBe(3);
    expect(movementSeconds('0','18446744073709551615')).toBe(3);
    expect(movementSeconds('1000','1050',0.25)).toBe(0.05);
  });
  it('keeps decimal-u64 clocks exact past the Number range',()=>{
    expect(movementSeconds('9007199254740992','9007199254740993')).toBe(0.001);
    expect(movementSeconds('9007199254740993','9007199254740993')).toBe(0.18);
    expect(movementSeconds('9007199254740993','9007199254740992')).toBe(0.18);
    expect(movementSeconds('0','9007199254740993')).toBe(3);
    expect(movementSeconds('18446744073709551615','18446744073709551615')).toBe(0.18);
  });
  it('falls back on clocks it cannot trust as canonical decimal text',()=>{
    for(const bad of ['','abc','007','1.5','-1','+12',' 12 ','1e3','12 ','1_000'])
      expect(movementSeconds(bad,'2000')).toBe(0.18);
    for(const bad of ['00','01','18446744073709551616','99999999999999999999999'])
      expect(movementSeconds('0',bad)).toBe(0.18);
    expect(movementSeconds('1000','nonsense',0.25)).toBe(0.25);
    expect(movementSeconds('1000','1000',0.25)).toBe(0.25);
  });
});
