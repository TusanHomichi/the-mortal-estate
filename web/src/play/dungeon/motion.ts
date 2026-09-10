import {cellKey} from './view';

/**
 * Presentation planning for dungeon combat and locomotion.
 *
 * Every input here is an already Rust-decoded event from an accepted state update; this module
 * narrows that data locally for animation only. It is not a wire validator, and it decides
 * nothing: not legality, not reachability, not timing, not readiness. Call `combatCues` once per
 * accepted state-update snapshot, never per command result, or acknowledged commands would be
 * animated twice. Nothing here mutates its inputs or issues commands.
 */

export type CombatClip = 'jab_left'|'jab_right'|'uppercut_right'|'hook_left'|'block_high'|'block_side'|'block_cover'|'block_lean'|'flying_kick';
export interface CombatCue { actorId:string; clip:CombatClip; faceActorId:string|null }

type Row = Readonly<Record<string,unknown>>;
const row = (value:unknown):Row|null => typeof value==='object'&&value!==null&&!Array.isArray(value)?value as Row:null;
const text = (value:unknown):string|null => typeof value==='string'?value:null;

/** The four accepted unarmed punches, in rotation order. */
const PUNCHES:readonly CombatClip[] = ['jab_left','jab_right','uppercut_right','hook_left'];
/** Presentation variety by incoming attack mode; the wire Blocked outcome names no block source. */
const BLOCKS:Readonly<Record<string,CombatClip>> = {fight:'block_high',kick:'block_side',jumpkick:'block_lean',poke:'block_cover'};

interface PhysicalFeedback { sourceId:string|null; targetId:string; mode:string; outcome:string }

/** `feedback` / `physical_combat` rows only, in the shape `crates/tme-protocol` serializes. */
function physicalFeedback(event:unknown):PhysicalFeedback|null {
  const envelope=row(event);
  if(!envelope||envelope['kind']!=='feedback')return null;
  const cue=row(envelope['cue']);
  if(!cue||cue['kind']!=='physical_combat')return null;
  const target=row(cue['target']),targetId=target?text(target['actor_id']):null;
  const source=cue['source']===null||cue['source']===undefined?null:row(cue['source']);
  const outcome=row(cue['outcome']),mode=text(cue['mode']);
  const outcomeKind=outcome?text(outcome['kind']):null;
  if(targetId===null||mode===null||outcomeKind===null)return null;
  return {sourceId:source?text(source['actor_id']):null,targetId,mode,outcome:outcomeKind};
}

/**
 * Attack and guard cues for the martial actor.
 *
 * Only hit, missed, and blocked swings are acted on: no_sight and not_ready mean the actor never
 * attacked. A swing animates its attacker regardless of which of those three it produced. Kick,
 * poke, shoot, and throw get no attacker clip because no accepted clip matches them. A block cue
 * needs a blocked outcome on the martial actor's side; the wire outcome collapses hand, shield,
 * and armor absorption into one word, so this is a generic cover pose, never a claim about which
 * source stopped the blow. `faceActorId` is only ever a visible counterpart; a hidden attacker is
 * null rather than a guessed direction.
 */
export function combatCues(events:readonly unknown[],martialActorId:string,visibleActorIds:ReadonlySet<string>,variation:number,unarmed:boolean):CombatCue[] {
  const cues:CombatCue[]=[];
  const visible=(id:string|null):string|null => id!==null&&visibleActorIds.has(id)?id:null;
  const martialVisible=martialActorId!==''&&visibleActorIds.has(martialActorId);
  // Variation seeds the punch rotation; each swing in this snapshot advances it.
  let rotation=Number.isFinite(variation)?Math.trunc(variation):0;
  for(const event of events){
    const hit=physicalFeedback(event);
    if(!hit||!martialVisible)continue;
    if(hit.outcome!=='hit'&&hit.outcome!=='missed'&&hit.outcome!=='blocked')continue;
    if(hit.sourceId===martialActorId){
      if(hit.mode==='jumpkick')cues.push({actorId:martialActorId,clip:'flying_kick',faceActorId:visible(hit.targetId)});
      else if(hit.mode==='fight'&&unarmed){
        const index=((rotation%PUNCHES.length)+PUNCHES.length)%PUNCHES.length;
        rotation+=1;
        cues.push({actorId:martialActorId,clip:PUNCHES[index]!,faceActorId:visible(hit.targetId)});
      }
    }
    // A self-directed row is the martial actor's own swing; the attack cue above already owns it.
    if(hit.targetId===martialActorId&&hit.sourceId!==martialActorId&&hit.outcome==='blocked'){
      const clip=BLOCKS[hit.mode];
      if(clip)cues.push({actorId:martialActorId,clip,faceActorId:visible(hit.sourceId)});
    }
  }
  return cues;
}

interface Place { realm:string; level:string; point:{x:number;y:number} }

function place(value:unknown):Place|null {
  const position=row(value);
  if(!position)return null;
  const realm=text(position['realm']),level=text(position['level']),coord=row(position['position']);
  if(realm===null||level===null||!coord)return null;
  const x=coord['x'],y=coord['y'];
  if(typeof x!=='number'||typeof y!=='number'||!Number.isInteger(x)||!Number.isInteger(y))return null;
  return {realm,level,point:{x,y}};
}

/**
 * The authoritative `actor_moved` chain for one actor, when it is a complete, visible, local path.
 *
 * Every point comes from the server; this never pathfinds and never inserts an intermediate
 * corner or a diagonal shortcut, so a rejected chain means no route at all rather than a guessed
 * one. A discontinuity, a realm or level change, an off-window point, or a traversal that leaves
 * the flat local floor (stairs, climb, pit, passage, portal) refuses the whole chain, as does a
 * chain that does not finish on the actor's actual cell. Walk, swim, and a local open door are
 * eligible. Returns the touched cells in travel order, or null.
 */
export function movementRoute(events:readonly unknown[],actorId:string,level:string,realm:string,visibleCells:ReadonlySet<string>,end:{x:number;y:number}):{x:number;y:number}[]|null {
  const route:{x:number;y:number}[]=[];
  let lastKey:string|null=null;
  for(const event of events){
    const moved=row(event);
    if(!moved||moved['kind']!=='actor_moved'||text(moved['actor_id'])!==actorId)continue;
    const navigation=text(moved['navigation']),from=place(moved['from']),to=place(moved['to']);
    if(!from||!to||(navigation!=='walk'&&navigation!=='swim'&&navigation!=='door'))return null;
    if(from.realm!==realm||to.realm!==realm||from.level!==level||to.level!==level)return null;
    const fromKey=cellKey(from.point),toKey=cellKey(to.point);
    if(!visibleCells.has(fromKey)||!visibleCells.has(toKey))return null;
    if(lastKey!==null&&lastKey!==fromKey)return null;
    if(lastKey===null)route.push({x:from.point.x,y:from.point.y});
    if(toKey!==fromKey)route.push({x:to.point.x,y:to.point.y});
    lastKey=toKey;
  }
  if(route.length<2)return null;
  if(cellKey(route[route.length-1]!)!==cellKey(end))return null;
  return route;
}

const DECIMAL=/^(0|[1-9][0-9]*)$/;
const U64_MAX=18446744073709551615n;
/**
 * DecimalU64 arrives as canonical decimal text: digits only, no leading zeros, within u64.
 * Anything else is untrusted, so it falls back rather than being parsed into a fake clock.
 * Parsing stays in BigInt so a clock past 2^53 never loses bits and never rounds down.
 */
const decimalMillis=(value:string):bigint|null => {
  if(typeof value!=='string'||!DECIMAL.test(value))return null;
  const parsed=BigInt(value);
  return parsed<=U64_MAX?parsed:null;
};

/**
 * Visual travel time for the observer's own movement, in seconds.
 *
 * The interval is the remaining time the server already granted, capped at 3s. Even a 1ms
 * remainder stays within that deadline. `fallback` (0.18s) is the
 * presentation-only visual lag when no positive interval is left; it is not an authoritative
 * movement cost, and the returned seconds never grant readiness — the server's `ready_at` and
 * `can_act` own that.
 */
export function movementSeconds(logicalTime:string,readyAt:string,fallback=0.18):number {
  const logical=decimalMillis(logicalTime),ready=decimalMillis(readyAt);
  if(logical===null||ready===null)return fallback;
  const remaining=ready-logical;
  if(remaining<=0n)return fallback;
  if(remaining>=3000n)return 3;
  return Number(remaining)/1000;
}
