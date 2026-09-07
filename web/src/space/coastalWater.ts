import { BufferAttribute, BufferGeometry, Color, Group, Mesh, ShaderMaterial, Vector3, type OrthographicCamera } from "three";
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
    result.x=.028*sin(a)+.015*sin(b)+.007*sin(c);
    result.yz=.028*1.45*cos(a)*d0+.015*2.35*cos(b)*d1+.007*3.8*cos(c)*d2;
    return result;
  }
`;
export const coastalWaterVertexShader = /* glsl */ `
  uniform float elapsed;
  uniform float seaHeight;
  attribute vec2 shore;
  varying vec3 vWaterPosition;
  varying vec2 vShore;
  ${waves}
  void main(){
    vShore=shore;
    float shelter=smoothstep(.2,2.5,shore.y);
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
  uniform vec3 sunColour;
  uniform vec3 skyColour;
  varying vec3 vWaterPosition;
  varying vec2 vShore;
  ${waves}
  void main(){
    vec2 p=vWaterPosition.xz;
    vec3 wave=swell(p,elapsed);
    float shelter=smoothstep(.2,2.5,vShore.y);
    vec2 ripple=vec2(
      .075*cos(dot(p,vec2(9.3,5.1))-elapsed*1.8),
      .055*cos(dot(p,vec2(-6.2,10.7))-elapsed*2.1)
    );
    vec3 n=normalize(vec3(-wave.y*shelter-ripple.x,1.0,-wave.z*shelter-ripple.y));
    float facing=clamp(dot(n,eyeDirection),0.0,1.0);
    float fresnel=.12+.7*pow(1.0-facing,3.0);
    float deep=smoothstep(.6,5.5,vShore.y);
    vec3 colour=mix(vec3(.018,.095,.09),vec3(.006,.025,.047),deep);
    float crest=smoothstep(-.01,.044,wave.x);
    colour*=.72+.5*max(dot(n,sunDirection),0.0);
    colour+=vec3(.007,.045,.047)*crest;
    vec3 reflection=reflect(-eyeDirection,n);
    float sun=pow(max(dot(reflection,sunDirection),0.0),80.0);
    vec2 skyUv=reflection.xz/max(.16,reflection.y)*4.0+elapsed*.013;
    float cloud=.5+.5*sin(skyUv.x+sin(skyUv.y*1.7))*cos(skyUv.y*.8);
    vec3 reflectedSky=mix(skyColour*.3,skyColour*1.6,smoothstep(.24,.78,cloud));
    colour=mix(colour,reflectedSky,fresnel)+sunColour*sun*.48;
    float glint=pow(max(dot(n,normalize(eyeDirection+vec3(-.2,1.0,.2))),0.0),70.0);
    colour+=skyColour*glint*.12;
    // The changing surface intersects the same bank mesh used for contact.
    float depth=vWaterPosition.y-vShore.x;
    float edge=(1.0-smoothstep(.012,.065,depth))*smoothstep(-.008,.016,depth);
    float lace=.5+.5*sin(p.x*19.0+p.y*7.0+sin(p.y*17.0-elapsed*.7));
    float wash=.6+.4*sin(elapsed*1.2+p.x*1.7+p.y*.8);
    float foam=edge*smoothstep(.18,.72,lace)*wash;
    colour=mix(colour,vec3(.38,.49,.46),foam*.66);
    gl_FragColor=vec4(colour,1.0);
    #include <tonemapping_fragment>
    #include <colorspace_fragment>
  }
`;

/** One opaque draw; no scene-copy, reflection camera, or per-frame CPU mesh work. */
export function addCoastalWater(group: Group, space: FeelSpace, surface: TerrainSurface,
  elapsed: {value:number}, palette: ScenePalette, camera: OrthographicCamera, sunDirection: Vector3): void {
  const data=buildWaterSurface(space,surface),geometry=new BufferGeometry();
  geometry.setAttribute("position",new BufferAttribute(new Float32Array(data.positions),3));
  geometry.setAttribute("shore",new BufferAttribute(new Float32Array(data.shore),2));
  geometry.setIndex(data.indices);geometry.computeBoundingSphere();
  const material=new ShaderMaterial({name:"CoastalWater",vertexShader:coastalWaterVertexShader,fragmentShader:coastalWaterFragmentShader,
    uniforms:{elapsed,seaHeight:{value:SEA_HEIGHT},eyeDirection:{value:camera.getWorldDirection(new Vector3()).negate()},
      sunDirection:{value:sunDirection.clone().normalize()},sunColour:{value:palette.key.clone()},
      skyColour:{value:new Color(.12,.19,.24)}}});
  const sea=new Mesh(geometry,material);sea.name="SeaBackdrop";sea.frustumCulled=false;group.add(sea);
}
