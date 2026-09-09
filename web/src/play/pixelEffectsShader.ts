// World effects use the scenery lattice; character samples retain their finer raster.
export const PIXEL_EFFECT_VERTEX=`
attribute vec2 position;
varying vec2 uv;
void main(){uv=vec2(position.x*.5+.5,.5-position.y*.5);gl_Position=vec4(position,0.,1.);}
`;
export const PIXEL_EFFECT_FRAGMENT=`
precision highp float;
varying vec2 uv;
uniform sampler2D colour,material,normals,grade;
uniform vec2 size,offset,sceneSize;
uniform vec3 ambient;
uniform vec4 weather;
uniform float time,lighting,scale,activeScene;
uniform vec4 lights[8];
uniform float strengths[8];
uniform vec4 shadows[16];
uniform vec2 shadowStyle[16];
float hash(vec2 p){return fract(sin(dot(p,vec2(127.1,311.7)))*43758.5453);}
float noise(vec2 p){vec2 i=floor(p),f=fract(p);f=f*f*(3.-2.*f);return mix(mix(hash(i),hash(i+vec2(1.,0.)),f.x),mix(hash(i+vec2(0.,1.)),hash(i+1.),f.x),f.y);}
vec3 lut(vec3 c){
 c=clamp(c,0.,1.);float b=c.b*15.;float lo=floor(b),hi=min(15.,lo+1.);
 vec2 a=vec2((lo*16.+c.r*15.+.5)/256.,(c.g*15.+.5)/16.);
 vec2 z=vec2((hi*16.+c.r*15.+.5)/256.,a.y);
 return mix(texture2D(grade,a).rgb,texture2D(grade,z).rgb,fract(b));
}
void main(){
 if(activeScene<.5){gl_FragColor=vec4(texture2D(colour,uv).rgb,1.);return;}
 vec2 pixel=floor(uv*size),world=floor(pixel/scale)-offset;
 vec4 m=texture2D(material,uv);
 vec2 sampleUV=uv;
 // Interior leaf regions alone may move. Requiring both mask samples prevents
 // pulling trunks, actors, buildings or the baked silhouette into the warp.
 float sway=sin(time*1.7+world.y*.09+sin(world.x*.04))*sin(time*.47+world.x*.023);
 float shift=floor(sway*weather.z*m.g+0.5);
 vec2 displaced=uv+vec2(shift*scale,0.)/size;
 if(m.g>.25 && texture2D(material,displaced).g>.25)sampleUV=displaced;
 vec3 base=texture2D(colour,sampleUV).rgb;
 float h=m.r*255.;vec3 normal=normalize(texture2D(normals,uv).rgb*2.-1.);
 vec3 p=vec3(world.x,world.y+h,h);
 vec3 illumination=ambient;
 for(int i=0;i<8;i++){
  vec4 l=lights[i];vec3 delta=vec3(l.x,l.y+l.z,l.z)-p;
  float distance=length(delta);float falloff=pow(max(0.,1.-distance/max(1.,l.w)),2.);
  // Front-facing surfaces darken rapidly when the light goes behind their base.
  float front=mix(1.,smoothstep(-24.,12.,delta.y),max(0.,normal.y));
  float diffuse=max(0.,dot(normal,normalize(delta+vec3(.001))));
  float flicker=.94+.035*sin(time*7.3+float(i)*2.1)+.025*sin(time*13.1+float(i));
  illumination+=vec3(1.,.64,.29)*falloff*strengths[i]*lighting*flicker*front*(.2+.8*diffuse);
 }
 float shadow=0.;
 for(int i=0;i<16;i++) {
  vec4 ray=shadows[i];vec2 q=world-ray.xy;
  float t=clamp(dot(q,ray.zw)/max(1.,dot(ray.zw,ray.zw)),0.,1.);
  float d=length(q-ray.zw*t);
  float width=shadowStyle[i].x*(1.-t*.7);
  shadow+=shadowStyle[i].y*(1.-smoothstep(width-1.,width+1.,d))*(1.-t*.5);
 }
 shadow=min(.38,shadow)*(1.-smoothstep(1.,12.,h));
 vec3 result=lut(base*illumination*(1.-shadow));
 // Preserve painted emitters locally; ordinary bright stone is not emissive.
 result=mix(result,base,m.b*.8);
 float lowFog=1.-smoothstep(10.,145.,h);
 float cloud=noise(vec2(world.x*.009-time*.019,world.y*.022+time*.007));
 float fog=weather.x*lowFog*smoothstep(.18,.85,cloud);
 result=mix(result,vec3(.65,.73,.72)*max(.35,ambient.r),fog);
 // Sparse world-phased streaks. No screen-attached sheet swimming with the camera.
 vec2 rainP=vec2(world.x+world.y*.23,world.y+time*190.);
 vec2 cell=floor(rainP/vec2(23.,47.)),f=mod(rainP,vec2(23.,47.));
 float rain=step(.001,weather.y)*step(hash(cell),weather.y*.42)*step(f.x,1.)*step(f.y,7.);
 result=mix(result,vec3(.66,.77,.84),rain*.32);
 bool outside=world.x<0.||world.y<0.||world.x>=sceneSize.x||world.y>=sceneSize.y;
 gl_FragColor=vec4(outside ? base : result,1.);
}
`;
