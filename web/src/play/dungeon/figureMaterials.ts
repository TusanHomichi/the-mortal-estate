import * as T from 'three';

/** Each figure owns its material instances; mesh geometry and textures stay shared. */
export class FigureMaterials {
  private readonly states = new Map<T.Material, {opacity:number;transparent:boolean;depthWrite:boolean}>();
  constructor(body:T.Object3D) {
    const copies=new Map<T.Material,T.Material>();
    const own=(source:T.Material):T.Material=>{
      let material=copies.get(source);
      if(!material){
        material=source.clone();copies.set(source,material);
        this.states.set(material,{opacity:material.opacity,transparent:material.transparent,depthWrite:material.depthWrite});
      }
      return material;
    };
    body.traverse(object=>{
      if(object instanceof T.Mesh)object.material=Array.isArray(object.material)?object.material.map(own):own(object.material);
    });
  }
  ghost(enabled:boolean):void {
    for(const [material,base] of this.states){
      material.opacity=enabled?base.opacity*.35:base.opacity;
      material.transparent=enabled||base.transparent;
      material.depthWrite=enabled?false:base.depthWrite;
      material.needsUpdate=true;
    }
  }
  dispose():void {for(const material of this.states.keys())material.dispose();this.states.clear();}
}
