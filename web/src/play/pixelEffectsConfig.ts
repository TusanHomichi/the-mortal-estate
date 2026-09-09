import type { PixelFile } from './pixelPacket';

export interface PixelLight { x:number; y:number; height:number; radius:number; strength:number }
export interface PixelShadow { x:number; y:number; height:number; radius:number }
export interface PixelEffectScene {
  material:PixelFile; normals:PixelFile; lights:PixelLight[]; shadows:PixelShadow[];
}
export interface PixelEffectProfile {
  ambient:[number,number,number]; fog:number; rain:number; wind:number; light:number;
  grade:PixelFile;
}
export interface PixelEffectsConfig {
  scenes:{arrival:PixelEffectScene;temple:PixelEffectScene};
  profiles:Record<string,PixelEffectProfile>;
  default_profile:string;
}
export function validatePixelEffects(value:PixelEffectsConfig):void {
  const bounded=(x:number,min:number,max:number)=>Number.isFinite(x)&&x>=min&&x<=max;
  if(!value?.scenes?.arrival || !value.scenes.temple || !value.profiles?.day || !value.profiles[value.default_profile])throw new Error('Missing pixel effects calibration.');
  for(const [name,scene] of Object.entries(value.scenes)) {
    const size=name==='temple' ? 512 : 4096;
    if(!scene.material||!scene.normals||!Array.isArray(scene.lights)||scene.lights.length>64||!Array.isArray(scene.shadows)||scene.shadows.length>32)throw new Error('Invalid pixel effect scene.');
    for(const c of scene.shadows)if(!bounded(c.x,0,size)||!bounded(c.y,0,size)||!bounded(c.height,1,255)||!bounded(c.radius,1,128))throw new Error('Invalid pixel shadow.');
    for(const l of scene.lights)if(!bounded(l.x,0,size)||!bounded(l.y,0,size)||!bounded(l.height,1,255)||
      !bounded(l.radius,1,512)||!bounded(l.strength,0,2))throw new Error('Invalid pixel light.');
  }
  for(const p of Object.values(value.profiles))if(!p.grade||!Array.isArray(p.ambient)||p.ambient.length!==3||
    !p.ambient.every(x=>bounded(x,0,1.5))||!bounded(p.fog,0,.65)||!bounded(p.rain,0,1)||
    !bounded(p.wind,0,2)||!bounded(p.light,0,2))throw new Error('Invalid pixel atmosphere profile.');
}
export function pixelEffectFiles(value:PixelEffectsConfig):PixelFile[] {
  return [...Object.values(value.scenes).flatMap(s=>[s.material,s.normals]),...Object.values(value.profiles).map(p=>p.grade)];
}
