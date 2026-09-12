import * as T from 'three';

/** Original seamless material grain; raster texture on standing 3D geometry. */
export function groundTexture():T.DataTexture {
  const size=128,data=new Uint8Array(size*size*4);
  const hash=(x:number,y:number)=>((Math.imul(x,1597334677)^Math.imul(y,3812015801))>>>0)%1000/1000;
  const noise=(x:number,y:number,period:number)=>{
    const ix=Math.floor(x),iy=Math.floor(y),fx=x-ix,fy=y-iy,sx=fx*fx*(3-2*fx),sy=fy*fy*(3-2*fy);
    const a=hash(ix%period,iy%period),b=hash((ix+1)%period,iy%period),c=hash(ix%period,(iy+1)%period),d=hash((ix+1)%period,(iy+1)%period);
    return (a*(1-sx)+b*sx)*(1-sy)+(c*(1-sx)+d*sx)*sy;
  };
  for(let y=0;y<size;y++)for(let x=0;x<size;x++){
    const grain=noise(x/16,y/16,8)*.6+noise(x/4,y/4,32)*.3+hash(x,y)*.1;
    const value=Math.round(180+grain*75),i=(y*size+x)*4;data.set([value,value,value,255],i);
  }
  const texture=new T.DataTexture(data,size,size,T.RGBAFormat);texture.wrapS=texture.wrapT=T.RepeatWrapping;
  texture.magFilter=T.LinearFilter;texture.minFilter=T.LinearMipmapLinearFilter;texture.generateMipmaps=true;texture.needsUpdate=true;
  return texture;
}
