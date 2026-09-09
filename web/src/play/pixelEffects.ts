import type { Coord } from '../authoritative/state';
import type { PixelPacket } from './pixelPacket';
import type { PixelEffectScene, PixelEffectProfile, PixelShadow } from './pixelEffectsConfig';
import { PIXEL_EFFECT_VERTEX, PIXEL_EFFECT_FRAGMENT } from './pixelEffectsShader';
import { PixelCompositor, pixelProgram, type PixelImage } from './pixelCompositor';

/** Presentation-only G-buffer and WebGL post pass. Geometry and observation stay upstream. */
export class PixelEffects {
  private readonly compositor:PixelCompositor;
  private readonly position:number;
  private gradeFile='';
  private readonly gl:WebGLRenderingContext;
  private readonly program:WebGLProgram;
  private readonly grade:WebGLTexture;
  private readonly buffer:WebGLBuffer;
  private readonly uniforms=new Map<string,WebGLUniformLocation|null>();
  private scene:PixelEffectScene|null=null;
  private sceneSize={x:512,y:512};
  private offset={x:0,y:0};
  private indoor=false;
  private scale=1;
  private lost=false;
  private actors:PixelShadow[]=[];
  readonly profileName:string;
  private readonly profile:PixelEffectProfile;
  private readonly onLost=(event:Event)=>{event.preventDefault();this.lost=true;};
  constructor(private readonly packet:PixelPacket,readonly canvas=document.createElement('canvas')) {
    const config=packet.manifest.effects;
    this.profileName=new URLSearchParams(location.search).get('atmosphere')??config.default_profile;
    const profile=config.profiles[this.profileName];
    if(!profile)throw new Error('Unknown pixel atmosphere profile.');
    this.profile=profile;
    const gl=this.canvas.getContext('webgl',{alpha:false,antialias:false,depth:false,stencil:false,preserveDrawingBuffer:true});
    if(!gl)throw new Error('Pixel atmosphere requires WebGL.');
    this.gl=gl;
    gl.disable(gl.DITHER); // Preserve exact repeated scenery pixels after colour grading.
    const program=pixelProgram(gl,PIXEL_EFFECT_VERTEX,PIXEL_EFFECT_FRAGMENT);
    this.program=program;gl.useProgram(program);
    const buffer=gl.createBuffer();if(!buffer)throw new Error('Pixel shader buffer unavailable.');this.buffer=buffer;
    gl.bindBuffer(gl.ARRAY_BUFFER,buffer);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([-1,-1,1,-1,-1,1,-1,1,1,-1,1,1]),gl.STATIC_DRAW);
    this.position=gl.getAttribLocation(program,'position');gl.enableVertexAttribArray(this.position);gl.vertexAttribPointer(this.position,2,gl.FLOAT,false,0,0);
    for(const [i,name] of ['colour','material','normals','grade'].entries())gl.uniform1i(this.uniform(name),i);
    const grade=gl.createTexture();if(!grade)throw new Error('Pixel grade texture unavailable.');this.grade=grade;
    gl.activeTexture(gl.TEXTURE3);gl.bindTexture(gl.TEXTURE_2D,grade);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
    this.uploadGrade(packet.images.get(profile.grade.file)!);this.gradeFile=profile.grade.file;
    this.compositor=new PixelCompositor(gl);
    this.canvas.addEventListener('webglcontextlost',this.onLost);
  }
  private uniform(name:string):WebGLUniformLocation|null {
    if(!this.uniforms.has(name))this.uniforms.set(name,this.gl.getUniformLocation(this.program,name));
    return this.uniforms.get(name)!;
  }
  private uploadGrade(image:TexImageSource):void {
    const gl=this.gl;gl.activeTexture(gl.TEXTURE3);gl.bindTexture(gl.TEXTURE_2D,this.grade);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
  }
  begin(level:string|null,width:number,height:number,offset:Coord,scale=1):void {
    const scene=level==='arrival'||level==='temple' ? this.packet.manifest.effects.scenes[level] : null;
    if(this.scene!==scene||this.scale!==scale)this.compositor.clearImages();
    this.scale=scale;this.scene=scene;this.actors=[];
    this.indoor=level==='temple';this.offset=offset;
    this.sceneSize=this.indoor ? {x:512,y:512} : {x:this.packet.manifest.exterior.width,y:this.packet.manifest.exterior.height};
    if(this.canvas.width!==width*scale||this.canvas.height!==height*scale){this.canvas.width=width*scale;this.canvas.height=height*scale;}
    this.compositor.begin(width,height,scale);
    if(scene)for(const [target,file] of [[1,scene.material.file],[2,scene.normals.file]] as const){
      const image=this.packet.images.get(file)!;this.compositor.image(target,image,[offset.x,offset.y,image.width,image.height]);
    }
  }
  paint(image:PixelImage,x:number,y:number,width=image.width,height=image.height,crop?:readonly [number,number,number,number]):void {
    this.compositor.image(0,image,[x,y,width,height],crop);
  }
  fill(colour:string,x:number,y:number,width:number,height:number):void {this.compositor.fill(colour,[x,y,width,height]);}
  invalidate(image:PixelImage):void {this.compositor.invalidate(image);}
  /** Restore lighting under the same foreground alpha and rectangle as colour. */
  foreground(x:number,y:number,width:number,height:number,alpha?:PixelImage):void {
    if(!this.scene)return;
    for(const [target,file] of [[1,this.scene.material.file],[2,this.scene.normals.file]] as const)
      this.compositor.image(target,this.packet.images.get(file)!,[x,y,width,height],[x-this.offset.x,y-this.offset.y,width,height],alpha);
  }
  actor(image:PixelImage,x:number,y:number,size:number,foot:Coord):void {
    this.paint(image,x,y,size,size);
    if(!this.scene)return;
    this.actors.push({x:foot.x-this.offset.x,y:foot.y-this.offset.y,height:Math.min(128,foot.y-y),radius:10});
    this.compositor.image(1,image,[x,y,size,size],undefined,undefined,1,foot.y-y);
    this.compositor.image(2,image,[x,y,size,size],undefined,undefined,2);
  }
  render(now:number):HTMLCanvasElement {
    if(this.lost||this.gl.isContextLost())throw new Error('Pixel atmosphere graphics context was lost. Reload to reconnect.');
    this.compositor.flush();
    const gl=this.gl,p=this.profile;
    gl.bindFramebuffer(gl.FRAMEBUFFER,null);gl.disable(gl.BLEND);
    gl.viewport(0,0,this.canvas.width,this.canvas.height);gl.useProgram(this.program);
    gl.bindBuffer(gl.ARRAY_BUFFER,this.buffer);gl.enableVertexAttribArray(this.position);gl.vertexAttribPointer(this.position,2,gl.FLOAT,false,0,0);
    for(let i=0;i<4;i++){gl.activeTexture(gl.TEXTURE0+i);gl.bindTexture(gl.TEXTURE_2D,i===3 ? this.grade : this.compositor.textures[i]!);}
    gl.uniform1f(this.uniform('activeScene'),this.scene ? 1 : 0);
    if(!this.scene){gl.drawArrays(gl.TRIANGLES,0,6);return this.canvas;}
    const grade=(this.indoor ? this.packet.manifest.effects.profiles.day! : p).grade.file;
    if(grade!==this.gradeFile){this.uploadGrade(this.packet.images.get(grade)!);this.gradeFile=grade;}
    gl.uniform2f(this.uniform('size'),this.canvas.width,this.canvas.height);gl.uniform1f(this.uniform('scale'),this.scale);gl.uniform2f(this.uniform('offset'),this.offset.x,this.offset.y);
    gl.uniform2f(this.uniform('sceneSize'),this.sceneSize.x,this.sceneSize.y);
    gl.uniform3fv(this.uniform('ambient'),this.indoor ? [.86,.82,.76] : p.ambient);
    gl.uniform4f(this.uniform('weather'),this.indoor ? 0 : p.fog,this.indoor ? 0 : p.rain,this.indoor ? 0 : p.wind,0);
    gl.uniform1f(this.uniform('time'),now/1000);gl.uniform1f(this.uniform('lighting'),this.indoor ? 1 : p.light);
    const centre={x:this.canvas.width/(2*this.scale)-this.offset.x,y:this.canvas.height/(2*this.scale)-this.offset.y};
    const distance=(l:{x:number;y:number})=>(l.x-centre.x)**2+(l.y-centre.y)**2;
    const selected=[...this.scene.lights].sort((a,b)=>distance(a)-distance(b)).slice(0,8),lights=new Float32Array(32),strengths=new Float32Array(8);
    selected.forEach((l,i)=>{lights.set([l.x,l.y,l.height,l.radius],i*4);strengths[i]=l.strength;});
    gl.uniform4fv(this.uniform('lights[0]'),lights);gl.uniform1fv(this.uniform('strengths[0]'),strengths);
    const rays=new Float32Array(64),styles=new Float32Array(32);
    const casters=[...this.actors,...[...this.scene.shadows].sort((a,b)=>distance(a)-distance(b))].slice(0,4);
    casters.forEach((caster,i)=>{
      const put=(slot:number,dx:number,dy:number,opacity:number)=>{
        rays.set([caster.x,caster.y,dx,dy],slot*4);styles.set([caster.radius,opacity],slot*2);
      };
      put(i*4,-caster.height*.35,caster.height*.20,this.indoor ? 0 : p.ambient[0]*.13);
      const nearest=[...this.scene!.lights].sort((a,b)=>((a.x-caster.x)**2+(a.y+a.height-caster.y)**2)-((b.x-caster.x)**2+(b.y+b.height-caster.y)**2)).slice(0,3);
      nearest.forEach((l,j)=>{
        const dx=caster.x-l.x,dy=caster.y-l.y-l.height,d=Math.hypot(dx,dy);
        const length=Math.min(90,caster.height*.6);
        put(i*4+j+1,dx/Math.max(1,d)*length,dy/Math.max(1,d)*length,Math.max(0,1-d/l.radius)*.15*l.strength*(this.indoor ? 1 : p.light));
      });
    });
    gl.uniform4fv(this.uniform('shadows[0]'),rays);gl.uniform2fv(this.uniform('shadowStyle[0]'),styles);
    gl.drawArrays(gl.TRIANGLES,0,6);
    return this.canvas;
  }
  dispose():void {
    this.compositor.dispose();
    this.canvas.removeEventListener('webglcontextlost',this.onLost);
    this.gl.deleteTexture(this.grade);
    this.gl.deleteBuffer(this.buffer);this.gl.deleteProgram(this.program);
    this.gl.getExtension('WEBGL_lose_context')?.loseContext();
  }
  clear():void {
    this.gl.bindFramebuffer(this.gl.FRAMEBUFFER,null);
    this.gl.clearColor(21/255,21/255,25/255,1);this.gl.clear(this.gl.COLOR_BUFFER_BIT);
  }
}
