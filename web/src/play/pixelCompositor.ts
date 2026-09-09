/** GPU-resident colour and lighting surfaces. Coordinates use logical top-left pixels. */
export type PixelImage = HTMLImageElement | HTMLCanvasElement;
type Rect = readonly [number,number,number,number];
interface Paint { image?:PixelImage; rect:Rect; crop?:Rect; mask?:PixelImage; colour?:readonly number[]; mode?:number; elevation?:number }
const vertex=`
attribute vec2 position;
uniform vec2 viewport;
uniform vec4 rect,crop;
varying vec2 uv,local;
void main(){local=position;uv=crop.xy+position*crop.zw;gl_Position=vec4((rect.xy+position*rect.zw)/viewport*2.-1.,0.,1.);}
`;
const fragment=`
precision highp float;
uniform sampler2D image,mask;
uniform vec4 colour;
uniform float mode,masked,elevation,height;
varying vec2 uv,local;
void main(){
 if(mode>2.5){gl_FragColor=colour;return;}
 vec4 sample=texture2D(image,uv);
 if(mode>.5){
  float a=sample.a;
  if(mode<1.5){float h=clamp(floor(elevation+.5),0.,255.)*clamp(1.-local.y*height/max(1.,elevation),0.,1.);sample=vec4(h/255.*a,0.,0.,a);}
  else sample=vec4(vec3(128./255.,1.,128./255.)*a,a);
 }
 if(masked>.5)sample*=texture2D(mask,local).a;
 gl_FragColor=sample;
}
`;
export function pixelProgram(gl:WebGLRenderingContext,vs:string,fs:string):WebGLProgram {
 const compile=(type:number,source:string)=>{
  const shader=gl.createShader(type);if(!shader)throw Error('Pixel shader allocation failed.');
  gl.shaderSource(shader,source);gl.compileShader(shader);
  if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS)){const reason=gl.getShaderInfoLog(shader);gl.deleteShader(shader);throw Error(`Pixel shader compilation failed: ${reason}`);}
  return shader;
 };
 const a=compile(gl.VERTEX_SHADER,vs),b=compile(gl.FRAGMENT_SHADER,fs),program=gl.createProgram();
 if(!program)throw Error('Pixel shader program unavailable.');
 gl.attachShader(program,a);gl.attachShader(program,b);gl.linkProgram(program);gl.deleteShader(a);gl.deleteShader(b);
 if(!gl.getProgramParameter(program,gl.LINK_STATUS)){const reason=gl.getProgramInfoLog(program);gl.deleteProgram(program);throw Error(`Pixel shader link failed: ${reason}`);}
 return program;
}
export class PixelCompositor {
 readonly textures:WebGLTexture[]=[];
 private readonly targets:WebGLFramebuffer[]=[];
 private readonly images=new Map<PixelImage,WebGLTexture>();
 private readonly queues:Paint[][]=[[],[],[]];
 private readonly program:WebGLProgram;
 private readonly vertices:WebGLBuffer;
 private readonly position:number;
 private readonly uniforms:Record<string,WebGLUniformLocation|null>;
 private readonly white=document.createElement('canvas');
 private width=0;private height=0;private scale=1;private pending=false;
 constructor(private readonly gl:WebGLRenderingContext) {
  this.white.width=1;this.white.height=1;const c=this.white.getContext('2d')!;c.fillStyle='white';c.fillRect(0,0,1,1);
  this.program=pixelProgram(gl,vertex,fragment);this.position=gl.getAttribLocation(this.program,'position');
  this.uniforms=Object.fromEntries(['viewport','rect','crop','colour','mode','masked','elevation','height','image','mask'].map(k=>[k,gl.getUniformLocation(this.program,k)]));
  const vertices=gl.createBuffer();if(!vertices)throw Error('Pixel geometry allocation failed.');this.vertices=vertices;
  gl.bindBuffer(gl.ARRAY_BUFFER,vertices);gl.bufferData(gl.ARRAY_BUFFER,new Float32Array([0,0,1,0,0,1,0,1,1,0,1,1]),gl.STATIC_DRAW);
  for(let i=0;i<3;i++){
   const texture=gl.createTexture(),target=gl.createFramebuffer();if(!texture||!target)throw Error('Pixel composition target unavailable.');
   this.textures.push(texture);this.targets.push(target);
   gl.bindTexture(gl.TEXTURE_2D,texture);this.sampling();
  }
 }
 private sampling():void {
  const gl=this.gl;
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.NEAREST);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
 }
 private texture(image:PixelImage):WebGLTexture {
  let texture=this.images.get(image);if(texture)return texture;
  const gl=this.gl;texture=gl.createTexture()??undefined;if(!texture)throw Error('Pixel image texture unavailable.');
  gl.bindTexture(gl.TEXTURE_2D,texture);this.sampling();
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL,true);
  gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
  this.images.set(image,texture);return texture;
 }
 begin(width:number,height:number,scale:number):void {
  const gl=this.gl;
  if(this.width!==width||this.height!==height||this.scale!==scale){
   this.width=width;this.height=height;this.scale=scale;
   for(let i=0;i<3;i++){
    gl.bindTexture(gl.TEXTURE_2D,this.textures[i]!);gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,width*scale,height*scale,0,gl.RGBA,gl.UNSIGNED_BYTE,null);
    gl.bindFramebuffer(gl.FRAMEBUFFER,this.targets[i]!);gl.framebufferTexture2D(gl.FRAMEBUFFER,gl.COLOR_ATTACHMENT0,gl.TEXTURE_2D,this.textures[i]!,0);
    if(gl.checkFramebufferStatus(gl.FRAMEBUFFER)!==gl.FRAMEBUFFER_COMPLETE)throw Error('Pixel composition framebuffer is incomplete.');
   }
   gl.bindFramebuffer(gl.FRAMEBUFFER,null);
  }
  this.queues.forEach(q=>{q.length=0;});this.pending=true;
 }
 image(target:number,image:PixelImage,rect:Rect,crop?:Rect,mask?:PixelImage,mode=0,elevation=0):void {
  if(rect[0]+rect[2]<=0||rect[1]+rect[3]<=0||rect[0]>=this.width||rect[1]>=this.height)return;
  this.queues[target]!.push({image,rect,crop,mask,mode,elevation});
 }
 fill(colour:string,rect:Rect):void {
  const hex=colour.slice(1),a=hex.length===8?parseInt(hex.slice(6),16)/255:1;
  this.queues[0]!.push({rect,colour:[0,2,4].map(i=>parseInt(hex.slice(i,i+2),16)/255*a).concat(a),mode:3});
 }
 flush():void {
  if(!this.pending)return;this.pending=false;
  const gl=this.gl,u=this.uniforms;gl.useProgram(this.program);
  gl.bindBuffer(gl.ARRAY_BUFFER,this.vertices);gl.enableVertexAttribArray(this.position);gl.vertexAttribPointer(this.position,2,gl.FLOAT,false,0,0);
  gl.viewport(0,0,this.width*this.scale,this.height*this.scale);
  gl.uniform2f(u.viewport!,this.width,this.height);gl.uniform1i(u.image!,0);gl.uniform1i(u.mask!,1);
  gl.enable(gl.BLEND);gl.blendFunc(gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
  // A complete sampler is required even by solid-colour branches on WebGL 1.
  const fallback=this.white;
  gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,this.texture(fallback));
  const clear=[[21/255,21/255,25/255,1],[0,0,0,1],[128/255,128/255,1,1]];
  for(let i=0;i<3;i++){
   gl.bindFramebuffer(gl.FRAMEBUFFER,this.targets[i]!);gl.clearColor(...clear[i]! as [number,number,number,number]);gl.clear(gl.COLOR_BUFFER_BIT);
   for(const cmd of this.queues[i]!){
    gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,this.texture(cmd.image??fallback));
    gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,this.texture(cmd.mask??fallback));
    const image=cmd.image??fallback,w=image.width,h=image.height,crop=cmd.crop??[0,0,w,h];
    gl.uniform4f(u.crop!,crop[0]/w,crop[1]/h,crop[2]/w,crop[3]/h);gl.uniform4f(u.rect!,...cmd.rect);
    gl.uniform4fv(u.colour!,cmd.colour??[1,1,1,1]);gl.uniform1f(u.mode!,cmd.mode??0);gl.uniform1f(u.masked!,cmd.mask?1:0);
    gl.uniform1f(u.elevation!,cmd.elevation??0);gl.uniform1f(u.height!,cmd.rect[3]);gl.drawArrays(gl.TRIANGLES,0,6);
   }
  }
  gl.disable(gl.BLEND);gl.bindFramebuffer(gl.FRAMEBUFFER,null);
 }
 invalidate(image:PixelImage):void {const texture=this.images.get(image);if(texture)this.gl.deleteTexture(texture);this.images.delete(image);}
 clearImages():void {for(const texture of this.images.values())this.gl.deleteTexture(texture);this.images.clear();}
 dispose():void {
  this.clearImages();for(const texture of this.textures)this.gl.deleteTexture(texture);
  for(const target of this.targets)this.gl.deleteFramebuffer(target);this.gl.deleteBuffer(this.vertices);this.gl.deleteProgram(this.program);
 }
}
