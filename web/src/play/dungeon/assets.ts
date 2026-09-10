import * as T from 'three';
import {GLTFLoader,type GLTF} from 'three/addons/loaders/GLTFLoader.js';
import receipt from './receipt.json';
import {verifySha256} from '../../assetDigest';
import {disposeSkeletons} from './rigResources';

export const COMBAT_CLIPS=['guard','walk','jab_left','jab_right','uppercut_right','hook_left',
  'block_high','block_side','block_cover','block_lean','flying_kick'] as const;
export interface FigureAsset { scene:T.Group; clips:readonly T.AnimationClip[]; idle:string; stride:number }
export interface DungeonAssets { fallback:FigureAsset; male:FigureAsset; female:FigureAsset }

export function disposeFigureAssets(assets:readonly FigureAsset[]):void {
  const geometries=new Set<T.BufferGeometry>(),materials=new Set<T.Material>(),textures=new Set<T.Texture>();
  for(const asset of assets)asset.scene.traverse(o=>{
    if(o instanceof T.Mesh){geometries.add(o.geometry);for(const m of Array.isArray(o.material)?o.material:[o.material]){
      materials.add(m);for(const value of Object.values(m))if(value instanceof T.Texture)textures.add(value);
    }}
  });
  textures.forEach(t=>t.dispose());materials.forEach(m=>m.dispose());geometries.forEach(g=>g.dispose());
  for(const asset of assets)disposeSkeletons(asset.scene);
}

/** Refuse a clip that would silently bind only part of a character skeleton. */
export function assertFigureClips(scene:T.Object3D,clips:readonly T.AnimationClip[],required:readonly string[]):void {
  for(const name of required){
    const matches=clips.filter(c=>c.name===name);
    if(matches.length!==1||!Number.isFinite(matches[0]!.duration)||matches[0]!.duration<=0||!matches[0]!.tracks.length)
      throw Error(`Dungeon animation missing or invalid: ${name}.`);
    for(const track of matches[0]!.tracks){
      const parsed=T.PropertyBinding.parseTrackName(track.name);
      if(!T.PropertyBinding.findNode(scene,parsed.nodeName))throw Error(`Dungeon animation bone missing: ${name}.`);
    }
  }
}

export async function loadDungeonBody():Promise<DungeonAssets> {
  const parsed:GLTF[]=[];
  const load=async(asset:{file:string;sha256:string})=>{
    const response=await fetch(`/feel-assets/${asset.file}`,{credentials:'omit',cache:'no-store'});
    if(!response.ok)throw Error('Dungeon figure unavailable.');
    const data=await response.arrayBuffer();await verifySha256(data,asset.sha256);
    const manager=new T.LoadingManager();let refused=false;
    manager.setURLModifier(url=>{if(!url.startsWith('blob:')){refused=true;throw Error('External dungeon model dependency.');}return url;});
    manager.onError=()=>{refused=true;};
    const gltf=await new GLTFLoader(manager).parseAsync(data,'');parsed.push(gltf);
    if(refused)throw Error('Dungeon model dependency refused.');
    return gltf;
  };
  try{
    // Sequential parsing lets refusal release every successfully decoded source.
    const body=await load(receipt.body),motion=await load(receipt.motion);
    assertFigureClips(body.scene,motion.animations,['idle']);
    const male=await load(receipt.martial.male),female=await load(receipt.martial.female);
    for(const asset of [male,female])assertFigureClips(asset.scene,asset.animations,COMBAT_CLIPS);
    return {fallback:{scene:body.scene,clips:motion.animations,idle:'idle',stride:1},
      male:{scene:male.scene,clips:male.animations,idle:'guard',stride:receipt.martial.male.stride},
      female:{scene:female.scene,clips:female.animations,idle:'guard',stride:receipt.martial.female.stride}};
  }catch(error){disposeFigureAssets(parsed.map(g=>({scene:g.scene,clips:g.animations,idle:'',stride:1})));throw error;}
}
