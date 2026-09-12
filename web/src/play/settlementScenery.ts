import * as T from 'three';
import type {Coord,Snapshot} from '../authoritative/state';
import {geography} from './worldSpace';
import {TILE} from './dungeon/view';
import type {SettlementAssets} from './settlementAssets';
import {groundTexture} from './groundTexture';

// PCFSoftShadowMap is a retired alias that mutates the renderer's mode on a
// shadow refresh. Cached frames must keep the same depth-sampler contract.
export const SETTLEMENT_SHADOW_MAP=T.PCFShadowMap;

type Member=typeof geography.members[number];
type Bounds={min:Coord;max:Coord};
const colors:Record<string,number>={expedition_forest:0x5c6941,expedition_grass:0x6a784b,
  expedition_path:0x9c8968,expedition_town_ground:0x858161,expedition_sand:0xb6a889,
  expedition_water:0x456f72,expedition_bridge:0x776047};
const inside=(p:Coord,b:Bounds)=>p.x>=b.min.x&&p.x<=b.max.x&&p.y>=b.min.y&&p.y<=b.max.y;

/** Static authored scenery has no actor, observation, passability or action authority. */
export class SettlementScenery {
  readonly group=new T.Group();
  occluders:T.Group[]=[];
  structures:string[]=[];
  private signature='';
  private generated=new Set<T.Mesh>();
  private readonly texture=groundTexture();
  constructor(private readonly assets:SettlementAssets){}

  present(snapshot:Snapshot):boolean {
    const level=snapshot.envelope.frame.observation_center.level;
    const context=snapshot.envelope.static_scene_context as {bounds:Bounds};
    return this.presentStatic(level,context.bounds);
  }
  presentStatic(level:string,bounds?:Bounds):boolean {
    const member=geography.members.find(m=>m.member===level);
    if(!member||!this.assets.has(level==='arrival'?'town_temple':`room_${level}`))throw Error('Unknown settlement scenery.');
    bounds??={min:{x:0,y:0},max:{x:member.width-1,y:member.height-1}};
    const signature=JSON.stringify([level,bounds]);if(this.signature===signature)return false;
    this.clear();this.signature=signature;
    if(level==='arrival')this.exterior(member,bounds);else this.interior(member);
    return true;
  }
  private instance(name:string,x:number,z:number):T.Group {
    const source=this.assets.get(name);if(!source)throw Error(`World model missing: ${name}.`);
    const root=source.clone(true);root.name=name;root.scale.set(TILE,1,TILE);root.position.set(x*TILE,0,z*TILE);
    this.group.add(root);return root;
  }
  private exterior(member:Member,bounds:Bounds):void {
    const batches=new Map<string,typeof member.cells>();
    for(const cell of member.cells.filter(c=>inside(c,bounds))) {
      const terrain=cell.terrain.find(t=>t.layer==='base_terrain')?.class??'expedition_town_ground';
      const rows=batches.get(terrain)??[];rows.push(cell);batches.set(terrain,rows);
    }
    const matrix=new T.Matrix4(),tone=new T.Color();
    for(const [terrain,cells] of batches) {
      const water=terrain==='expedition_water',bridge=terrain==='expedition_bridge';
      const material=new T.MeshStandardMaterial({color:colors[terrain]??colors.expedition_town_ground,map:water?null:this.texture,
        roughness:water?.35:.97,metalness:water?.15:0});
      const floor=new T.InstancedMesh(new T.BoxGeometry(TILE,.07,TILE),material,cells.length);
      floor.receiveShadow=true;this.generated.add(floor);this.group.add(floor);
      cells.forEach((cell,index)=>{
        matrix.makeTranslation(cell.x*TILE,water?-.16:bridge?.025:-.045,cell.y*TILE);floor.setMatrixAt(index,matrix);
        const variation=((cell.x*73856093^cell.y*19349663)>>>0)%13;
        floor.setColorAt(index,tone.setScalar(.95+variation*.007));
      });
      floor.instanceMatrix.needsUpdate=true;
      if(bridge)for(const cell of cells)for(let n=0;n<6;n++)this.box(TILE-.025,.035,TILE/6-.025,0x8a7050,cell.x*TILE,.08,(cell.y-.5+(n+.5)/6)*TILE);
    }
    for(const structure of member.structures) {
      if(structure.x>bounds.max.x+.5||structure.x+structure.width-1<bounds.min.x-.5||
        structure.y>bounds.max.y+.5||structure.y+structure.height-1<bounds.min.y-.5)continue;
      const transition=member.transitions.find(t=>t.access.x===structure.access.x&&t.access.y===structure.access.y);
      if(!transition)throw Error('Authored building lacks its entrance.');
      const root=this.instance(`town_${transition.target_member}`,structure.x+(structure.width-1)/2,structure.y+(structure.height-1)/2);
      this.occluders.push(root);this.structures.push(structure.id);
    }
    // Forest is passable. Trunks sit at cell corners, never at walking anchors.
    for(const cell of member.cells.filter(c=>inside(c,bounds)&&c.terrain.some(t=>t.class==='expedition_forest'))) {
      if((cell.x*7+cell.y*11)%3)continue;
      const name=['managed_tree','managed_tree_broad','managed_tree_slim'][(cell.x+cell.y)%3]!;
      const tree=this.instance(name,cell.x+.43,cell.y+.43);tree.scale.set(.65*TILE,.65,.65*TILE);
      tree.rotation.y=(cell.x*13+cell.y*7)%4*Math.PI/2;this.occluders.push(tree);
    }
    const center={x:(bounds.min.x+bounds.max.x)/2,y:(bounds.min.y+bounds.max.y)/2};
    this.group.add(new T.HemisphereLight(0xe0e8dc,0x595041,1.4));
    const sun=new T.DirectionalLight(0xffe0ab,2.5);sun.position.set((center.x-8)*TILE,18,(center.y-6)*TILE);
    sun.target.position.set(center.x*TILE,0,center.y*TILE);sun.castShadow=true;sun.shadow.mapSize.set(2048,2048);
    Object.assign(sun.shadow.camera,{left:-22,right:22,top:22,bottom:-22,near:1,far:70});
    sun.shadow.bias=-.0003;sun.shadow.normalBias=.03;sun.shadow.autoUpdate=false;sun.shadow.needsUpdate=true;
    this.group.add(sun,sun.target);
  }
  private interior(member:Member):void {
    const room=this.instance(`room_${member.member}`,0,0);room.updateMatrixWorld(true);
    // Material-batched room meshes remain separate occlusion surfaces, so a
    // foreground furnishing does not fade the entire room and its floor.
    for(const child of [...room.children])if(child instanceof T.Mesh){
      const surface=new T.Group();surface.name=child.name;room.add(surface);surface.add(child);this.occluders.push(surface);
    }
    this.structures=[`room_${member.member}`];
    this.group.add(new T.HemisphereLight(0xffead1,0x53452f,.9));
    const markers:T.Object3D[]=[];room.traverse(o=>{if(o.userData.tme_light)markers.push(o);});
    for(const marker of markers.slice(0,8)) {
      const spec=marker.userData.tme_light as {intensity:number;distance:number};
      const light=new T.PointLight(0xffd393,spec.intensity*5,spec.distance*TILE,2);
      light.position.copy(marker.getWorldPosition(new T.Vector3()));this.group.add(light);
    }
    const key=new T.DirectionalLight(0xffecd5,1.2);key.position.set(2,10,member.height*TILE);this.group.add(key);
  }
  private box(w:number,h:number,d:number,color:number,x:number,y:number,z:number):void {
    const mesh=new T.Mesh(new T.BoxGeometry(w,h,d),new T.MeshStandardMaterial({color,roughness:1}));
    mesh.position.set(x,y,z);mesh.receiveShadow=true;this.generated.add(mesh);this.group.add(mesh);
  }
  clear():void {
    for(const mesh of this.generated){mesh.geometry.dispose();(mesh.material as T.Material).dispose();if(mesh instanceof T.InstancedMesh)mesh.dispose();}
    this.generated.clear();this.group.traverse(o=>{if(o instanceof T.DirectionalLight||o instanceof T.PointLight){o.shadow?.map?.dispose();o.dispose();}});
    this.group.clear();this.occluders=[];this.structures=[];this.signature='';
  }
  dispose():void {this.clear();this.texture.dispose();}
}
