import * as T from 'three';
import type {Frame} from '../authoritative/state';
import {TILE} from './dungeon/view';

/** Unique boundaries of observed walkable squares, independent of floor artwork. */
export function gridEdges(frame:Frame):number[][] {
  const edges=new Map<string,number[]>();
  for(const tile of frame.tiles){
    if(!tile.terrain_id||tile.passable!==true)continue;
    const x=tile.position.x,y=tile.position.y;
    for(const edge of [[x-.5,y-.5,x+.5,y-.5],[x-.5,y+.5,x+.5,y+.5],
      [x-.5,y-.5,x-.5,y+.5],[x+.5,y-.5,x+.5,y+.5]])edges.set(edge.join(':'),edge);
  }
  return [...edges.values()];
}
export class WorldGrid {
  readonly mesh=new T.LineSegments(new T.BufferGeometry(),new T.LineBasicMaterial({color:0xd8cba6,transparent:true,opacity:.16,depthWrite:false}));
  private signature='';
  present(frame:Frame):void {
    const edges=gridEdges(frame),signature=JSON.stringify(edges);if(signature===this.signature)return;
    this.signature=signature;this.mesh.geometry.dispose();
    this.mesh.geometry=new T.BufferGeometry().setFromPoints(edges.flatMap(([x,y,u,v])=>[
      new T.Vector3(x!*TILE,.055,y!*TILE),new T.Vector3(u!*TILE,.055,v!*TILE)]));
  }
  clear():void {this.mesh.geometry.dispose();this.mesh.geometry=new T.BufferGeometry();this.signature='';}
  dispose():void {this.mesh.geometry.dispose();(this.mesh.material as T.Material).dispose();}
}
