import * as T from 'three';
import {GLTFLoader,type GLTF} from 'three/addons/loaders/GLTFLoader.js';
import {clone} from 'three/addons/utils/SkeletonUtils.js';
import receipt from './receipt.json';
import {disposeSkeletons} from './rigResources';
import {verifySha256} from '../../assetDigest';
import type {Frame} from '../../authoritative/state';
import {occupantAnchors,TILE} from './view';

export async function loadDungeonBody():Promise<{body:GLTF;motion:GLTF}> {
  const load=async(asset:{file:string;sha256:string})=>{
    const response=await fetch(`/feel-assets/${asset.file}`,{credentials:'omit',cache:'no-store'});
    if(!response.ok)throw Error('Dungeon figure unavailable.');
    const data=await response.arrayBuffer();await verifySha256(data,asset.sha256);
    // Receipt-bound self-contained GLBs may not fetch additional dependencies.
    const manager=new T.LoadingManager();manager.setURLModifier(url=>{if(!url.startsWith('blob:'))throw Error('External dungeon model dependency.');return url;});
    return new GLTFLoader(manager).parseAsync(data,'');
  };
  const [body,motion]=await Promise.all([load(receipt.body),load(receipt.motion)]);
  if(!motion.animations.some(c=>c.name==='idle'))throw Error('Dungeon idle animation missing.');
  return {body,motion};
}
interface Figure {root:T.Group;body:T.Object3D;mixer:T.AnimationMixer;label:T.Sprite}
export class DungeonActors {
  readonly group=new T.Group();
  private figures=new Map<string,Figure>();
  constructor(private readonly assets:Awaited<ReturnType<typeof loadDungeonBody>>){}
  present(frame:Frame):void {
    const anchors=occupantAnchors(frame),active=new Set<string>();
    for(const row of frame.actors){
      if(row.life_state==='dead')continue;
      active.add(row.actor_id);let figure=this.figures.get(row.actor_id);
      if(!figure){
        const root=new T.Group(),body=clone(this.assets.body.scene);root.add(body);
        body.traverse(o=>{if(o instanceof T.Mesh){o.castShadow=false;o.receiveShadow=true;}});
        const mixer=new T.AnimationMixer(body);mixer.clipAction(this.assets.motion.animations.find(c=>c.name==='idle')!).play();
        const raster=document.createElement('canvas');raster.width=256;raster.height=40;
        const ink=raster.getContext('2d')!;ink.font='22px Georgia';ink.textAlign='center';ink.fillStyle='#ead7b0';ink.shadowColor='#000';ink.shadowBlur=4;ink.fillText(row.name,128,28,250);
        const label=new T.Sprite(new T.SpriteMaterial({map:new T.CanvasTexture(raster),transparent:true,depthTest:false,toneMapped:false}));
        label.position.y=2.0;label.scale.set(1.65,.26,1);label.renderOrder=25;root.add(label);
        figure={root,body,mixer,label};this.figures.set(row.actor_id,figure);this.group.add(root);
      }
      const at=anchors.get(row.actor_id)!;figure.root.position.set(at.x*TILE,0,at.y*TILE);figure.root.userData.actorId=row.actor_id;
    }
    for(const [id,figure] of this.figures)if(!active.has(id)){this.remove(figure);this.figures.delete(id);}
  }
  update(dt:number):void {for(const f of this.figures.values())f.mixer.update(dt);}
  bodies():T.Object3D[]{return [...this.figures.values()].map(f=>f.body);}
  pick(ray:T.Raycaster):string|null {
    const hit=ray.intersectObjects(this.bodies(),true)[0];if(!hit)return null;
    let o:T.Object3D|null=hit.object;while(o&&!o.userData.actorId)o=o.parent;return o?.userData.actorId??null;
  }
  private remove(f:Figure):void {f.mixer.stopAllAction();f.mixer.uncacheRoot(f.body);disposeSkeletons(f.body);f.root.removeFromParent();f.label.material.map?.dispose();f.label.material.dispose();}
  clear():void {for(const f of this.figures.values())this.remove(f);this.figures.clear();}
  dispose():void {this.clear();this.assets.body.scene.traverse(o=>{if(o instanceof T.Mesh){o.geometry.dispose();for(const m of Array.isArray(o.material)?o.material:[o.material]){for(const value of Object.values(m))if(value instanceof T.Texture)value.dispose();m.dispose();}}});}
}
