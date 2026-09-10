import {Object3D,SkinnedMesh,Skeleton} from 'three';
/** Clones own skeleton textures; geometry and material remain owned by the asset. */
export function disposeSkeletons(root:Object3D):void {
  const skeletons=new Set<Skeleton>();
  root.traverse(o=>{if(o instanceof SkinnedMesh)skeletons.add(o.skeleton);});
  for(const skeleton of skeletons)skeleton.dispose();
}
