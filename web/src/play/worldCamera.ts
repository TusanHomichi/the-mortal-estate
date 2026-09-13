import * as T from 'three';
import type {Coord} from '../authoritative/state';
import {TILE} from './dungeon/view';

/** Same fixed perspective as the dungeon, with room for exterior roof silhouettes. */
export function fitSettlementCamera(camera:T.PerspectiveCamera,center:Coord,width:number,height:number):void {
  const direction=new T.Vector3(0,Math.sin(55*Math.PI/180),Math.cos(55*Math.PI/180));
  const target=new T.Vector3(center.x*TILE,.65,center.y*TILE);
  camera.fov=20;camera.aspect=width/height;
  const distance=9*TILE/(2*Math.tan(camera.fov*Math.PI/360))*Math.max(1,1/camera.aspect);
  camera.position.copy(target).addScaledVector(direction,distance);camera.lookAt(target);
  camera.updateMatrixWorld(true);camera.updateProjectionMatrix();
}
