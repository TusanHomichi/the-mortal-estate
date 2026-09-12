import * as T from 'three';
import {DungeonOcclusion} from '../src/play/dungeon/occlusion';

/** Synthetic geometry only: measure composed transparency, without a server. */
export function measureOcclusion(){
 const canvas=document.createElement('canvas');document.body.append(canvas);
 const renderer=new T.WebGLRenderer({canvas,antialias:false,preserveDrawingBuffer:true});renderer.setSize(64,64);
 const scene=new T.Scene(),camera=new T.PerspectiveCamera(40,1,.1,100);camera.position.set(0,0,5);camera.lookAt(0,0,0);camera.updateMatrixWorld(true);
 const body=new T.Mesh(new T.PlaneGeometry(2,2),new T.MeshBasicMaterial({color:0x00ff00,toneMapped:false}));
 const head=new T.Bone();head.name='Head';body.add(head);scene.add(body);
 const roof=new T.Group();scene.add(roof);
 for(const z of [1,1.1,1.2]){
  const mesh=new T.Mesh(new T.PlaneGeometry(2,2),new T.MeshBasicMaterial({color:0xff0000,toneMapped:false}));mesh.position.z=z;roof.add(mesh);
 }
 const occlusion=new DungeonOcclusion();occlusion.bind([roof]);scene.updateMatrixWorld(true);
 const read=()=>{renderer.render(scene,camera);const gl=renderer.getContext(),pixel=new Uint8Array(4);gl.readPixels(32,32,1,1,gl.RGBA,gl.UNSIGNED_BYTE,pixel);return [...pixel];};
 const opaque=read();occlusion.update(camera,[body]);const faded=read();
 occlusion.update(camera,[]);const restored=read();occlusion.clear();const cleared=read();
 scene.traverse(o=>{if(o instanceof T.Mesh){o.geometry.dispose();o.material.dispose();}});renderer.dispose();canvas.remove();
 return {opaque,faded,restored,cleared};
}
