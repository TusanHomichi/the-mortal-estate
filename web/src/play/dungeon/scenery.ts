import * as T from 'three';
import type {Frame} from '../../authoritative/state';
import {TILE,cellKey,isWall,doorState} from './view';
import {wallParts,type WallPart} from './topology';

const floorColors:Record<string,number>={expedition_dungeon_water:0x294b56,expedition_fire_floor:0x9c3e16,
  expedition_dark_floor:0x202127,expedition_grass:0x425534,expedition_dirt:0x67503a,
  expedition_frozen_floor:0x7faaaa,expedition_wooden_floor:0x6d4c30,expedition_rubble:0x575651};
/** At most the observed 7x7 cells. No remembered hidden room meshes. */
export class DungeonScenery {
  readonly group=new T.Group();
  occluders:T.Group[]=[];
  parts:WallPart[]=[];
  torchCount=0;
  private signature='';
  private readonly brick=new T.CanvasTexture(this.bricks());
  private bricks():HTMLCanvasElement {
    const image=document.createElement('canvas');image.width=192;image.height=256;
    const c=image.getContext('2d')!;c.fillStyle='#8d918a';c.fillRect(0,0,192,256);
    for(let y=0;y<8;y++)for(let x=-1;x<3;x++){
      const at=x*96+(y%2)*48;c.fillStyle=`rgb(${132+y%3*4},${136+y%3*4},${129+y%3*4})`;c.fillRect(at+1,y*32+1,94,30);
      c.strokeStyle='#575951';c.strokeRect(at,y*32,96,32);
    }
    image.getContext('2d')!.imageSmoothingEnabled=false;return image;
  }
  present(frame:Frame):boolean {
    const signature=JSON.stringify([frame.observation_center,frame.tiles]);
    if(signature===this.signature)return false;
    this.clear();this.signature=signature;this.parts=wallParts(frame);
    const stone=new T.MeshStandardMaterial({color:0x93998f,map:this.brick,roughness:1});
    const cap=new T.MeshStandardMaterial({color:0x99998a,roughness:.9});
    const wood=new T.MeshStandardMaterial({color:0x60422a,roughness:.87});
    const iron=new T.MeshStandardMaterial({color:0x343630,metalness:.65,roughness:.65});
    const box=(parent:T.Object3D,w:number,h:number,d:number,material:T.Material,x:number,y:number,z:number)=>{
      const m=new T.Mesh(new T.BoxGeometry(w,h,d),material);m.position.set(x,y,z);m.castShadow=true;m.receiveShadow=true;parent.add(m);return m;
    };
    for(const tile of frame.tiles){
      const p=tile.position,x=p.x*TILE,z=p.y*TILE,id=tile.terrain_id!;
      const wall=isWall(id),door=doorState(tile);
      if(!wall&&!(door?.navigation==='door'&&!door.door_open)){
        const material=new T.MeshStandardMaterial({color:floorColors[id]??0x65625a,roughness:1});
        const floor=box(this.group,TILE-.015,.08,TILE-.015,material,x,-.045,z);floor.name=`floor:${cellKey(p)}`;
        if(id==='expedition_stairs_up'||id==='expedition_stairs_down')for(let i=0;i<5;i++){
          const step=id==='expedition_stairs_up'?i:4-i;
          box(this.group,TILE*.75,.08+step*.12,TILE*.15,cap,x,.04+step*.06,z+.5-i*.24);
        }
      }
    }
    for(const part of this.parts){
      const group=new T.Group(),mid=(part.start+part.end)*TILE/2;
      group.name=part.id;group.position.set(part.axis==='x'?mid:part.plane*TILE,0,part.axis==='x'?part.plane*TILE:mid);
      group.rotation.y=part.axis==='x'?0:Math.PI/2;this.group.add(group);this.occluders.push(group);
      const length=(part.end-part.start)*TILE;
      if(!part.door)box(group,length,part.height-.09,.20,stone,0,(part.height-.09)/2,0);
      else {
        box(group,.23,2.15,.24,cap,-.71,1.075,0);box(group,.23,2.15,.24,cap,.71,1.075,0);
        box(group,TILE,part.height-.09-2.115,.20,stone,0,(part.height-.09+2.115)/2,0);
        const hinge=new T.Group();hinge.position.x=-.59;hinge.rotation.y=part.open?-Math.PI/2:0;group.add(hinge);
        box(hinge,1.17,2.05,.09,wood,.595,1.04,0);
        for(const y of [.40,1.65])box(hinge,1.17,.085,.13,iron,.595,y,0);
        box(hinge,.08,.15,.16,iron,1.04,1.08,.02);
      }
      // Every adjoining segment owns the same continuous cap profile.
      box(group,length,.09,.235,cap,0,part.height-.045,0);
    }
    // Provisional fixed wall positions, independent of player movement. Nearby
    // light work is bounded; hidden/opposite-facing sources never enter the scene.
    const center=frame.observation_center.position;
    const torches=this.parts.filter(p=>!p.door&&p.axis==='x'&&p.tile.y<center.y&&p.tile.x%3===0)
      .sort((a,b)=>(a.tile.x-center.x)**2+(a.tile.y-center.y)**2-((b.tile.x-center.x)**2+(b.tile.y-center.y)**2)).slice(0,3);
    for(const p of torches){
      const x=p.tile.x*TILE,z=p.plane*TILE+.29;
      box(this.group,.09,.4,.09,wood,x,1.4,z).castShadow=false;box(this.group,.14,.06,.3,iron,x,1.28,z-.1).castShadow=false;
      const fire=new T.Mesh(new T.ConeGeometry(.07,.22,7),new T.MeshBasicMaterial({color:0xffbd58}));fire.position.set(x,1.7,z);this.group.add(fire);
      const light=new T.PointLight(0xffc18a,11,7,2);light.position.set(x,1.7,z+.05);light.castShadow=true;
      light.shadow.mapSize.set(256,256);light.shadow.bias=-.001;light.shadow.camera.near=.1;light.shadow.camera.far=7;
      light.shadow.autoUpdate=false;light.shadow.needsUpdate=true;this.group.add(light);
    }
    this.torchCount=torches.length;
    this.group.add(new T.HemisphereLight(0xffd3a0,0x423022,torches.length?.55:0));
    return true;
  }
  clear():void {
    const materials=new Set<T.Material>();this.group.traverse(o=>{
      if(o instanceof T.Mesh){o.geometry.dispose();for(const m of Array.isArray(o.material)?o.material:[o.material])materials.add(m);}
      if(o instanceof T.PointLight){o.shadow.map?.dispose();o.dispose();}
    });for(const m of materials)m.dispose();this.group.clear();this.occluders=[];this.parts=[];this.torchCount=0;this.signature='';
  }
  dispose():void {this.clear();this.brick.dispose();}
}
