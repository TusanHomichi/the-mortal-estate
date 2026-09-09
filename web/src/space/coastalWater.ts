import { BufferAttribute, BufferGeometry, Color, Group, Mesh, ShaderMaterial, Vector3, type OrthographicCamera } from "three";
import { buildSeabedSurface } from "../seabedSurface";
import { buildWaterSurface } from "../waterSurface";
import { SEA_HEIGHT, type TerrainSurface } from "../terrainSurface";
import type { FeelSpace } from "../feelTypes";
import type { ScenePalette } from "./palette";

const waves = /* glsl */ `
  // Height and analytical horizontal derivatives of three crossing swells.
  vec3 swell(vec2 p, float t) {
    vec3 result = vec3(0.0);
    vec2 d0=normalize(vec2(.85,.35)), d1=normalize(vec2(-.4,.9)), d2=normalize(vec2(.6,-.8));
    float a=dot(p,d0)*1.45-t*.85;
    float b=dot(p,d1)*2.35-t*1.13;
    float c=dot(p,d2)*3.8-t*1.55;
    result.x=.13*sin(a)+.065*sin(b)+.025*sin(c);
    result.yz=.13*1.45*cos(a)*d0+.065*2.35*cos(b)*d1+.025*3.8*cos(c)*d2;
    return result;
  }
`;
export const coastalWaterVertexShader = /* glsl */ `
  uniform float elapsed;
  uniform float seaHeight;
  attribute vec2 shore;
  attribute float exposure;
  varying float vExposure;
  varying vec3 vWaterPosition;
  varying vec2 vShore;
  ${waves}
  void main(){
    vShore=shore; vExposure=exposure;
    float shelter=smoothstep(.04,1.8,seaHeight-shore.x)*mix(.08,1.0,exposure);
    vec3 p=position;
    p.y=seaHeight+swell(p.xz,elapsed).x*shelter;
    vWaterPosition=p;
    gl_Position=projectionMatrix*modelViewMatrix*vec4(p,1.0);
  }
`;
export const coastalWaterFragmentShader = /* glsl */ `
  uniform float elapsed;
  uniform vec3 eyeDirection;
  uniform vec3 sunDirection;

  uniform vec3 skyColour;
  varying vec3 vWaterPosition;
  varying vec2 vShore;
  varying float vExposure;
  uniform float seaHeight;
  uniform float lightLevel;
  ${waves}
  void main(){
    vec2 p=vWaterPosition.xz;
    vec3 wave=swell(p,elapsed);
    float depth=max(0.0,vWaterPosition.y-vShore.x);
    float shelter=smoothstep(.04,1.8,seaHeight-vShore.x)*mix(.08,1.0,vExposure);
    vec2 ripple=vec2(
      .075*cos(dot(p,vec2(9.3,5.1))-elapsed*1.8),
      .055*cos(dot(p,vec2(-6.2,10.7))-elapsed*2.1)
    )*mix(.04,1.0,shelter);
    vec3 n=normalize(vec3(-wave.y*shelter-ripple.x,1.0,-wave.z*shelter-ripple.y));
    float facing=clamp(dot(n,eyeDirection),0.0,1.0);
    float fresnel=.02+.98*pow(1.0-facing,5.0);
    float deep=smoothstep(.25,3.5,depth);
    vec3 colour=mix(vec3(.035,.32,.27),vec3(.006,.075,.12),deep);
    float crest=smoothstep(.015,.18,wave.x)*shelter;
    colour*=.72+.5*max(dot(n,sunDirection),0.0);
    colour+=vec3(.025,.12,.10)*crest;
    vec3 reflection=reflect(-eyeDirection,n);

    vec3 reflectedSky=skyColour*mix(.35,.75,smoothstep(-.1,1.0,reflection.y));
    colour=mix(colour,reflectedSky,fresnel);
    // Broad sky glints model the changing slopes; the smaller sun lobe stays
    // tied to the same key direction as the town's cast shadows.
    float skyGlint=pow(max(reflection.y,0.0),18.0);
    float sunGlint=pow(max(dot(reflection,sunDirection),0.0),64.0);
    colour+=vec3(.15,.23,.24)*skyGlint*mix(.18,1.0,shelter);
    colour+=vec3(.65,.57,.39)*sunGlint;
    // The changing surface intersects the same bank mesh used for contact.

    float edge=(1.0-smoothstep(.012,.065,depth))*smoothstep(-.008,.016,depth);
    float lace=.5+.5*sin(p.x*19.0+p.y*7.0+sin(p.y*17.0-elapsed*.7));
    float wash=.6+.4*sin(elapsed*1.2+p.x*1.7+p.y*.8);
    float foam=edge*smoothstep(.18,.72,lace)*wash;
    colour=mix(colour,vec3(.38,.49,.46),foam*.66);
    // Eye-path absorption leaves the shelf readable, then conceals the bottom
    // smoothly in the channel. Reflection alone must not turn it into milk.
    float absorption=1.0-exp(-depth*.43/max(facing,.25));
    float alpha=clamp(.025+absorption*.97+fresnel*.35+foam*.35,0.0,.995);
    gl_FragColor=vec4(colour*lightLevel,alpha);
    #include <tonemapping_fragment>
    #include <colorspace_fragment>
  }
`;

/** One transparent sea and one opaque seabed; no scene-copy or reflection pass. */
export function addCoastalWater(group: Group, space: FeelSpace, surface: TerrainSurface,
  elapsed: {value:number}, palette: ScenePalette, camera: OrthographicCamera, sunDirection: Vector3): void {
  const data=buildWaterSurface(space,surface),geometry=new BufferGeometry();
  geometry.setAttribute("position",new BufferAttribute(new Float32Array(data.positions),3));
  geometry.setAttribute("exposure",new BufferAttribute(new Float32Array(data.exposure),1));
  geometry.setAttribute("shore",new BufferAttribute(new Float32Array(data.shore),2));
  geometry.setIndex(data.indices);geometry.computeBoundingSphere();
  const material=new ShaderMaterial({name:"CoastalWater",transparent:true,depthWrite:false,vertexShader:coastalWaterVertexShader,fragmentShader:coastalWaterFragmentShader,
    uniforms:{elapsed,lightLevel:{value:palette.keyIntensity*.3+palette.ambientIntensity*.2},seaHeight:{value:SEA_HEIGHT},eyeDirection:{value:camera.getWorldDirection(new Vector3()).negate()},
      sunDirection:{value:sunDirection.clone().normalize()},
      skyColour:{value:new Color(.12,.19,.24)}}});
  const bottom=new BufferGeometry();
  const bed=buildSeabedSurface(data,surface);
  bottom.setAttribute("position",new BufferAttribute(new Float32Array(bed.positions),3));
  bottom.setIndex(bed.indices); bottom.computeBoundingSphere();
  const bottomMaterial=new ShaderMaterial({name:"CoastalSeabed",uniforms:{
    lightLevel:{value:palette.ambientIntensity*.34+palette.keyIntensity*.22},
    elapsed, seaHeight:{value:SEA_HEIGHT}, sunlight:{value:palette.keyIntensity}},
    vertexShader:`varying vec3 p; void main(){p=position;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.0);}`,
    fragmentShader:`uniform float lightLevel; uniform float elapsed; uniform float seaHeight; uniform float sunlight; varying vec3 p;
      vec2 hash(vec2 q){return fract(sin(vec2(dot(q,vec2(127.1,311.7)),dot(q,vec2(269.5,183.3))))*43758.5453);}

      void main(){
        float grain=fract(sin(dot(floor(p.xz*23.0),vec2(127.1,311.7)))*43758.5453);
        float bed=.5+.5*sin(p.x*2.1+sin(p.z*1.7))*cos(p.z*1.4-p.x*.6);
        vec3 sand=mix(vec3(.33,.32,.23),vec3(.60,.53,.37),bed)*(.97+.04*grain);
        vec2 q=p.xz*3.3,cell=floor(q); float pebble=0.0; float rim=0.0;
        for(int j=-1;j<=1;j++)for(int i=-1;i<=1;i++){
          vec2 tile=cell+vec2(float(i),float(j)),random=hash(tile);
          vec2 offset=(q-tile-.15-random*.7)*vec2(1.0,.72+random.x*.6);
          float radius=.065+random.y*.12;
          float d=max(abs(offset.x*.92+offset.y*.26),max(abs(offset.y*.92-offset.x*.12),abs(offset.x*.62-offset.y*.68)))/radius;
          float present=step(.48,random.x);
          pebble=max(pebble,(1.0-smoothstep(.65,1.0,d))*present);
          rim=max(rim,(1.0-smoothstep(.9,1.25,d))*present);
        }
        sand=mix(sand,sand*.79,rim);
        sand=mix(sand,vec3(.33,.37,.29)*(.84+.28*grain),pebble);
        // A restrained animated light web on the actual bed, fading below the
        // lit shelf. It adds no texture, scene copy or render target.
        vec2 flow=p.xz*6.2+vec2(sin(p.z*.8+elapsed*.35),cos(p.x*.7-elapsed*.28))*.32;
        float folds=abs(sin(flow.x+sin(flow.y+elapsed*.45))
          +sin(flow.y-sin(flow.x-elapsed*.38)));
        float caustic=(1.0-smoothstep(.045,.21,folds))
          *smoothstep(.015,.12,seaHeight-p.y)*exp(-max(0.0,seaHeight-p.y)*.8);
        sand+=vec3(.065,.075,.045)*caustic*smoothstep(1.2,2.7,sunlight);
        gl_FragColor=vec4(sand*lightLevel,1.0);
        #include <tonemapping_fragment>
        #include <colorspace_fragment>
      }`});
  const seabed=new Mesh(bottom,bottomMaterial);seabed.name="CoastalSeabed";group.add(seabed);
  const sea=new Mesh(geometry,material);sea.name="SeaBackdrop";sea.renderOrder=-1;sea.frustumCulled=false;group.add(sea);
}
