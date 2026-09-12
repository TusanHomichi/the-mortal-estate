import * as T from 'three';
/** Fade the nearest obstructing surface once, including overlapping roof pieces. */
export class DungeonOcclusion {
  private entries:{group:T.Group;mesh:T.Mesh;original:T.Material|T.Material[];faded:T.Material|T.Material[];depth:T.Mesh;order:number}[]=[];
  private ray=new T.Raycaster();
  count=0;
  bind(groups:T.Group[]):void {
    this.clear();
    for(const group of groups){
      const meshes:T.Mesh[]=[];group.traverse(o=>{if(o instanceof T.Mesh)meshes.push(o);});
      for(const o of meshes){
      if(!o.geometry.boundingBox)o.geometry.computeBoundingBox();
      const original=o.material;
      const copy=(material:T.Material)=>{const result=material.clone();result.onBeforeCompile=material.onBeforeCompile;result.customProgramCacheKey=material.customProgramCacheKey;return result;};
      const fade=(material:T.Material)=>{const faded=copy(material);faded.transparent=true;faded.opacity=.4;faded.depthWrite=false;faded.depthFunc=T.EqualDepth;faded.forceSinglePass=true;return faded;};
      const faded=Array.isArray(original)?original.map(fade):fade(original);
      const prepass=(material:T.Material)=>{const depth=copy(material);depth.transparent=true;depth.colorWrite=false;depth.depthWrite=true;depth.depthFunc=T.LessEqualDepth;depth.blending=T.NoBlending;depth.forceSinglePass=true;return depth;};
      const depth=new T.Mesh(o.geometry,Array.isArray(original)?original.map(prepass):prepass(original));
      depth.name='occlusion-depth';depth.renderOrder=90;depth.visible=false;depth.raycast=()=>{};o.add(depth);
      this.entries.push({group,mesh:o,original,faded,depth,order:o.renderOrder});
      }
    }
  }
  update(camera:T.Camera,bodies:T.Object3D[]):void {
    const groups=new Set<T.Group>(),meshes=this.entries.map(e=>e.mesh),owners=new Map(this.entries.map(e=>[e.mesh,e.group]));
    for(const body of bodies){
      const samples:T.Object3D[]=[];
      body.traverse(o=>{if(o instanceof T.Bone&&/Hips|Spine|Head|Arm$|ForeArm$|Hand$|Leg$|Foot$/.test(o.name))samples.push(o);});
      for(const bone of samples){
      const target=bone.getWorldPosition(new T.Vector3()),screen=target.clone().project(camera);
      this.ray.setFromCamera(new T.Vector2(screen.x,screen.y),camera);this.ray.far=this.ray.ray.origin.distanceTo(target)-.03;
      for(const hit of this.ray.intersectObjects(meshes,false))groups.add(owners.get(hit.object as T.Mesh)!);
      }
    }
    for(const e of this.entries){const faded=groups.has(e.group);e.mesh.material=faded?e.faded:e.original;e.mesh.renderOrder=faded?91:e.order;e.depth.visible=faded;}
    this.count=groups.size;
  }
  clear():void {
    for(const e of this.entries){e.mesh.material=e.original;e.mesh.renderOrder=e.order;e.depth.removeFromParent();
      for(const value of [e.faded,e.depth.material])for(const m of Array.isArray(value)?value:[value])m.dispose();
    }
    this.entries=[];this.count=0;
  }
}
