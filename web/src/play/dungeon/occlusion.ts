import * as T from 'three';
/** Restore opaque masonry when it no longer blocks any observed body. */
export class DungeonOcclusion {
  private entries:{group:T.Group;mesh:T.Mesh;original:T.Material;faded:T.Material}[]=[];
  private ray=new T.Raycaster();
  count=0;
  bind(groups:T.Group[]):void {
    this.clear();
    for(const group of groups)group.traverse(o=>{if(o instanceof T.Mesh){
      const original=o.material as T.Material,faded=original.clone();faded.transparent=true;faded.opacity=.4;faded.depthWrite=false;
      this.entries.push({group,mesh:o,original,faded});
    }});
  }
  update(camera:T.Camera,bodies:T.Object3D[]):void {
    const groups=new Set<T.Group>(),meshes=this.entries.map(e=>e.mesh),owners=new Map(this.entries.map(e=>[e.mesh,e.group]));
    for(const body of bodies)for(const name of ['Hips','Spine','Head']){
      const bone=body.getObjectByName(name);if(!bone)continue;
      const target=bone.getWorldPosition(new T.Vector3()),screen=target.clone().project(camera);
      this.ray.setFromCamera(new T.Vector2(screen.x,screen.y),camera);this.ray.far=this.ray.ray.origin.distanceTo(target)-.03;
      for(const hit of this.ray.intersectObjects(meshes,false))groups.add(owners.get(hit.object as T.Mesh)!);
    }
    for(const e of this.entries)e.mesh.material=groups.has(e.group)?e.faded:e.original;
    this.count=groups.size;
  }
  clear():void {for(const e of this.entries){e.mesh.material=e.original;e.faded.dispose();}this.entries=[];this.count=0;}
}
