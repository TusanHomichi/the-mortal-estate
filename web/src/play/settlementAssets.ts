import * as T from 'three';
import {GLTFLoader} from 'three/addons/loaders/GLTFLoader.js';
import {verifySha256} from '../assetDigest';
import {assertEmbeddedStructure} from '../embeddedStructure';
import {disposeFigureAssets} from './dungeon/assets';
import receipt from './settlementReceipt.json';
import promotion from '../../../content/lands/first-expedition/promotion.json';

export type SettlementAssets=Map<string,T.Group>;
export function disposeSettlementAssets(assets:SettlementAssets):void {
  disposeFigureAssets([...assets.values()].map(scene=>({scene,clips:[],idle:'',stride:1})));
  assets.clear();
}
/** Only self-contained, receipt-bound original models enter the playable scene. */
export async function loadSettlementAssets():Promise<SettlementAssets> {
  if(receipt.status!=='candidate'||receipt.geography_master_sha256!==promotion.master.sha256)
    throw Error('World models and authored geography disagree.');
  const result:SettlementAssets=new Map();
  try {
    for(const [name,asset] of Object.entries(receipt.models)) {
      const response=await fetch(`/feel-assets/${asset.file}`,{credentials:'omit',cache:'no-store'});
      if(!response.ok)throw Error('World model unavailable.');
      const bytes=await response.arrayBuffer();await verifySha256(bytes,asset.sha256);assertEmbeddedStructure(bytes);
      const model=await new GLTFLoader().parseAsync(bytes,'');result.set(name,model.scene);
      let meshes=0;model.scene.traverse(o=>{if(o instanceof T.Mesh){meshes++;o.castShadow=true;o.receiveShadow=true;}});
      const bounds=new T.Box3().setFromObject(model.scene),size=bounds.getSize(new T.Vector3());
      if(!meshes||bounds.isEmpty()||![size.x,size.y,size.z].every(n=>Number.isFinite(n)&&n>0&&n<64))
        throw Error('Invalid world model geometry.');
    }
    return result;
  }catch(error){disposeSettlementAssets(result);throw error;}
}
