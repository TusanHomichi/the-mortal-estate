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

/** Exercise repeated cached-shadow frames, with the actual settlement setting. */
export async function measureShadowCycle(mode){
 const {SETTLEMENT_SHADOW_MAP}=await import('../src/play/settlementScenery');
 mode??=SETTLEMENT_SHADOW_MAP;
 const renderer=new T.WebGLRenderer({antialias:false,preserveDrawingBuffer:true});renderer.setSize(64,64);
 renderer.shadowMap.enabled=true;renderer.shadowMap.autoUpdate=false;renderer.shadowMap.needsUpdate=true;
 const scene=new T.Scene(),camera=new T.PerspectiveCamera(40,1,.1,100);camera.position.set(0,0,5);camera.lookAt(0,0,0);camera.updateMatrixWorld(true);
 const sun=new T.DirectionalLight(0xffffff,3);sun.position.set(1,2,5);sun.castShadow=true;sun.shadow.mapSize.set(32,32);sun.shadow.autoUpdate=false;sun.shadow.needsUpdate=true;scene.add(sun,new T.AmbientLight(0xffffff,1));
 const body=new T.Mesh(new T.PlaneGeometry(2,2),new T.MeshBasicMaterial({color:0x00ff00,toneMapped:false}));const head=new T.Bone();head.name='Head';body.add(head);scene.add(body);
 const roof=new T.Group();scene.add(roof);
 for(const z of [1,1.1,1.2]){
  const mesh=new T.Mesh(new T.PlaneGeometry(2,2),new T.MeshStandardMaterial({color:0xff0000,roughness:1,toneMapped:false}));mesh.position.z=z;mesh.castShadow=true;mesh.receiveShadow=true;roof.add(mesh);
 }
 const occlusion=new DungeonOcclusion();occlusion.bind([roof]);scene.updateMatrixWorld(true);const errors=[];
 const read=()=>{renderer.shadowMap.type=mode;renderer.render(scene,camera);const gl=renderer.getContext(),pixel=new Uint8Array(4);gl.readPixels(32,32,1,1,gl.RGBA,gl.UNSIGNED_BYTE,pixel);errors.push(gl.getError());return [...pixel];};
 const opaque=read(),cachedOpaque=read();occlusion.update(camera,[body]);const faded=read();occlusion.update(camera,[]);const restored=read();
 occlusion.clear();const cleared=read();
 scene.traverse(o=>{if(o instanceof T.Mesh){o.geometry.dispose();o.material.dispose();}});sun.shadow.map?.dispose();sun.dispose();renderer.dispose();
 return {opaque,cachedOpaque,faded,restored,cleared,errors};
}
