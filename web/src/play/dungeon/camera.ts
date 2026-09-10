import * as T from 'three';
import type {Coord} from '../../authoritative/state';
import {TILE,WALL_HEIGHT} from './view';
/** Fixed direction, seven ground rows/columns, same framing at every floor. */
export function fitDungeonCamera(camera:T.PerspectiveCamera,center:Coord,width:number,height:number):void {
  const a=55*Math.PI/180, direction=new T.Vector3(0,Math.sin(a),Math.cos(a));
  const target=new T.Vector3(center.x*TILE,.4,center.y*TILE);
  camera.position.copy(target).addScaledVector(direction,20);camera.lookAt(target);camera.updateMatrixWorld(true);
  const corners:T.Vector3[]=[];
  for(const x of [-3.5,3.5])for(const z of [-3.5,3.5])for(const y of [0,WALL_HEIGHT])corners.push(new T.Vector3((center.x+x)*TILE,y,(center.y+z)*TILE));
  const local=corners.map(p=>p.clone().applyMatrix4(camera.matrixWorldInverse));
  camera.fov=20;camera.aspect=width/height;
  const tangent=Math.tan(camera.fov*Math.PI/360);
  const distance=Math.max(...local.map(p=>p.z+20+1.06*Math.max(Math.abs(p.y)/tangent,Math.abs(p.x)/(tangent*camera.aspect))));
  camera.position.copy(target).addScaledVector(direction,distance);camera.updateMatrixWorld(true);camera.updateProjectionMatrix();
  const midpoint=new T.Box3().setFromPoints(corners.map(p=>p.clone().project(camera))).getCenter(new T.Vector3());
  camera.projectionMatrix.elements[8]!+=midpoint.x;camera.projectionMatrix.elements[9]!+=midpoint.y;
  camera.projectionMatrixInverse.copy(camera.projectionMatrix).invert();
}
