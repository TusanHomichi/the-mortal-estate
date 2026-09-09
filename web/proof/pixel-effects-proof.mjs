import assert from 'node:assert/strict';
import {writeFile,mkdir} from 'node:fs/promises';
import {startVite,launchProofBrowser,PROOF_ENGINES} from './serve.mjs';
const [packetPath,output]=process.argv.slice(2);if(!packetPath||!output)throw Error('Usage: pixel-effects-proof.mjs EXTERNAL_PACKET EXTERNAL_OUTPUT');
await mkdir(output,{recursive:true});const server=await startVite(packetPath);const reports=[];
try {
 for(const [name,engine] of Object.entries(PROOF_ENGINES)) {
  const launched=await launchProofBrowser({name,engine,executablePath:engine.executablePath()});
  try {
   const context=launched.context||await launched.browser.newContext();const page=await context.newPage();
   await page.route('**/__pixel-effects-proof__*',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><body style="margin:0;background:#151519"></body>'}));
   await page.goto(server.baseUrl+'__pixel-effects-proof__');
   const result=await page.evaluate(async()=>{
    const {PixelEffects}=await import('/src/play/pixelEffects.ts');
    const {loadPixelPacket}=await import('/src/play/pixelPacket.ts');
    const packet=await loadPixelPacket();
    const read=canvas=>{const copy=document.createElement('canvas');copy.width=canvas.width;copy.height=canvas.height;const c=copy.getContext('2d');c.drawImage(canvas,0,0);return c.getImageData(0,0,copy.width,copy.height).data;};
    const render=(packet,profile,level='arrival',time=0)=>{
     history.replaceState(null,'','?atmosphere='+profile);
     const effect=new PixelEffects(packet),source=document.createElement('canvas');source.width=1280;source.height=800;
     const c=source.getContext('2d');c.imageSmoothingEnabled=false;c.scale(2,2);c.fillStyle='#151519';c.fillRect(0,0,640,400);
     const offset=level==='arrival' ? {x:-306,y:0} : {x:64,y:0};
     c.drawImage(packet.images.get(level==='arrival' ? packet.manifest.exterior.background.file : packet.manifest.background.file),offset.x,offset.y);
     effect.begin(level,640,400,offset,2);
     effect.paint(source,0,0,640,400);
     const canvas=effect.render(time),pixels=read(canvas),image=canvas.toDataURL();effect.dispose();return {pixels,image};
    };
    const pictures={},profileBlocks={};
    for(const profile of ['day','dusk','night','rain','fog']) {
     const r=render(packet,profile);pictures[profile]=r.image;let errors=0;
     for(let y=0;y<800;y+=2)for(let x=0;x<1280;x+=2)for(let dy=0;dy<2;dy++)for(let dx=0;dx<2;dx++)for(let k=0;k<3;k++)if(r.pixels[(y*1280+x)*4+k]!==r.pixels[((y+dy)*1280+x+dx)*4+k])errors++;
     profileBlocks[profile]=errors;
    }
    pictures.temple=render(packet,'day','temple').image;
    // Isolate wind on the actual authored foliage map: fixed roots/silhouettes
    // and unmasked ground must not move when only time changes.
    const windPacket={...packet,manifest:structuredClone(packet.manifest)};
    windPacket.manifest.effects.profiles.day={...windPacket.manifest.effects.profiles.day,fog:0,rain:0,wind:2,light:0};
    windPacket.manifest.effects.scenes.arrival.lights=[];windPacket.manifest.effects.scenes.arrival.shadows=[];
    const a=render(windPacket,'day','arrival',0).pixels,b=render(windPacket,'day','arrival',3700).pixels;
    const mask=document.createElement('canvas');mask.width=1280;mask.height=800;const mc=mask.getContext('2d');mc.imageSmoothingEnabled=false;mc.scale(2,2);mc.drawImage(packet.images.get(packet.manifest.effects.scenes.arrival.material.file),-306,0);
    const material=mc.getImageData(0,0,1280,800).data;let leafChanges=0,outsideChanges=0,blockErrors=0;const outsidePixels=[];
    for(let i=0;i<a.length;i+=4)if(a[i]!==b[i]||a[i+1]!==b[i+1]||a[i+2]!==b[i+2]){if(material[i+1]>63)leafChanges++;else {outsideChanges++;if(outsidePixels.length<20)outsidePixels.push({x:i/4%1280,y:Math.floor(i/4/1280),g:material[i+1],a:Array.from(a.slice(i,i+3)),b:Array.from(b.slice(i,i+3))});}}
    for(let y=0;y<800;y+=2)for(let x=0;x<1280;x+=2)for(let dy=0;dy<2;dy++)for(let dx=0;dx<2;dx++)for(let k=0;k<3;k++)if(a[(y*1280+x)*4+k]!==a[((y+dy)*1280+x+dx)*4+k])blockErrors++;
    // The same interior at the same instant is insulated from outdoor presets.
    const day=render(packet,'day','temple',1000).pixels,rain=render(packet,'rain','temple',1000).pixels;
    let indoorDifferences=0;for(let i=0;i<day.length;i++)if(day[i]!==rain[i])indoorDifferences++;
    // Small controlled surfaces exercise the real compiled shader, independently
    // of the authored painting: height-aware fog and front/back light response.
    const makeImage=async draw=>{const c=document.createElement('canvas');c.width=128;c.height=128;draw(c.getContext('2d'));const i=new Image();i.src=c.toDataURL();await i.decode();return i;};
    const gray=await makeImage(c=>{c.fillStyle='#606060';c.fillRect(0,0,128,128);});
    const heights=await makeImage(c=>{c.fillStyle='#000000';c.fillRect(0,0,64,128);c.fillStyle='#a00000';c.fillRect(64,0,64,128);});
    const ground=await makeImage(c=>{c.fillStyle='#8080ff';c.fillRect(0,0,128,128);});
    const fixture={manifest:structuredClone(packet.manifest),images:new Map(packet.images)};
    fixture.images.set('fixture-colour.png',gray);fixture.images.set('fixture-material.png',heights);fixture.images.set('fixture-normal.png',ground);
    fixture.manifest.exterior.background.file='fixture-colour.png';fixture.manifest.effects.scenes.arrival={material:{file:'fixture-material.png'},normals:{file:'fixture-normal.png'},lights:[],shadows:[]};
    fixture.manifest.effects.profiles.day={...packet.manifest.effects.profiles.day,ambient:[1,1,1],fog:0,rain:0,wind:0,light:0};
    function small(time){history.replaceState(null,'','?atmosphere=day');const effect=new PixelEffects(fixture);const c=document.createElement('canvas');c.width=128;c.height=128;c.getContext('2d').drawImage(gray,0,0);effect.begin('arrival',128,128,{x:0,y:0});effect.paint(c,0,0,128,128);const out=read(effect.render(time));effect.dispose();return out;}
    const clear=small(0);fixture.manifest.effects.profiles.day.fog=.65;const fog=small(0);let lowFog=0,highFog=0;
    for(let y=0;y<128;y++)for(let x=0;x<128;x++){const i=(y*128+x)*4,d=Math.abs(clear[i]-fog[i]);if(x<64)lowFog+=d;else highFog+=d;}
    fixture.images.set('fixture-material.png',await makeImage(c=>{c.fillStyle='#280000';c.fillRect(0,0,128,128);}));
    fixture.images.set('fixture-normal.png',await makeImage(c=>{c.fillStyle='#80ff80';c.fillRect(0,0,128,128);}));
    fixture.manifest.effects.profiles.day={...fixture.manifest.effects.profiles.day,ambient:[.1,.1,.1],fog:0,light:1};
    fixture.manifest.effects.scenes.arrival.lights=[{x:64,y:96,height:40,radius:150,strength:1.5}];const front=small(0);
    fixture.manifest.effects.scenes.arrival.lights[0].y=32;const back=small(0);const sample=(64*128+64)*4;
    // Exercise the real composition surface: orientation, source cropping,
    // premultiplied overlap, retained uploads, invalidation and target lifetime.
    const effect=new PixelEffects(packet),tile=document.createElement('canvas'),alpha=document.createElement('canvas');
    tile.width=4;tile.height=4;alpha.width=1;alpha.height=1;
    const tc=tile.getContext('2d');for(const [x,y,colour] of [[0,0,'#ff0000'],[2,0,'#00ff00'],[0,2,'#0000ff'],[2,2,'#ffffff']]){tc.fillStyle=colour;tc.fillRect(x,y,2,2);}
    alpha.getContext('2d').fillStyle='rgba(255,0,0,0.5)';alpha.getContext('2d').fillRect(0,0,1,1);
    let uploads=0,deletedTargets=0;const gl=effect.gl,upload=gl.texImage2D,remove=gl.deleteFramebuffer;
    gl.texImage2D=function(...args){uploads++;return upload.apply(this,args);};
    gl.deleteFramebuffer=function(...args){deletedTargets++;return remove.apply(this,args);};
    function compose(scale=2){effect.begin(null,32,24,{x:0,y:0},scale);effect.paint(tile,0,0,4,4);effect.paint(tile,8,0,4,4,[2,0,2,2]);effect.fill('#0000ff',8,8,4,4);effect.paint(alpha,8,8,4,4);effect.fill('#00ff00',9,9,1,1);return read(effect.render(0));}
    const pixels=compose();const pixel=(data,x,y,scale=2)=>Array.from(data.slice(((y*scale)*(32*scale)+x*scale)*4,((y*scale)*(32*scale)+x*scale)*4+3));
    const orientation=[pixel(pixels,0,0),pixel(pixels,3,0),pixel(pixels,0,3),pixel(pixels,3,3)];
    const crop=pixel(pixels,9,1),overlap=pixel(pixels,8,8),order=pixel(pixels,9,9);
    uploads=0;compose();const reusedUploads=uploads;
    tc.fillStyle='#ffff00';tc.fillRect(0,0,4,4);effect.invalidate(tile);uploads=0;const updated=compose(),invalidatedUploads=uploads;
    const resized=compose(3),resizeSize=[effect.canvas.width,effect.canvas.height],resizedPixel=pixel(resized,0,0,3);
    effect.dispose();
    const composition={orientation,crop,overlap,order,reusedUploads,invalidatedUploads,updated:pixel(updated,0,0),resizeSize,resizedPixel,deletedTargets};
    return {composition,profileBlocks,outsidePixels,pictures,leafChanges,outsideChanges,blockErrors,indoorDifferences,lowFog,highFog,frontLight:front[sample],backLight:back[sample]};
   });
   await writeFile(`${output}/${name}-measurements.json`,JSON.stringify({...result,pictures:undefined},null,2));
   for(const count of Object.values(result.profileBlocks))assert.equal(count,0);
   assert(result.leafChanges>100);assert.equal(result.outsideChanges,0);assert.equal(result.blockErrors,0);assert.equal(result.indoorDifferences,0);
   assert(result.lowFog>1000);assert.equal(result.highFog,0);assert(result.frontLight>result.backLight+10);
   assert.deepEqual(result.composition.orientation,[[255,0,0],[0,255,0],[0,0,255],[255,255,255]]);
   assert.deepEqual(result.composition.crop,[0,255,0]);assert.deepEqual(result.composition.order,[0,255,0]);
   assert(Math.abs(result.composition.overlap[0]-128)<=1&&result.composition.overlap[1]===0&&Math.abs(result.composition.overlap[2]-127)<=1);
   assert.equal(result.composition.reusedUploads,0);assert.equal(result.composition.invalidatedUploads,1);
   assert.deepEqual(result.composition.updated,[255,255,0]);assert.deepEqual(result.composition.resizedPixel,[255,255,0]);
   assert.deepEqual(result.composition.resizeSize,[96,72]);assert.equal(result.composition.deletedTargets,3);
   for(const [profile,url] of Object.entries(result.pictures))await writeFile(`${output}/${name}-${profile}.png`,Buffer.from(url.split(',')[1],'base64'));
   delete result.pictures;reports.push({engine:name,rendering:launched.rendering,adapter:launched.renderer,...result});console.log('PASS pixel effects',name);
  }finally{await launched.stop();}
 }
 await writeFile(`${output}/verification.json`,JSON.stringify({verdict:'PASS',reports},null,2)+'\n');
}finally{await server.stop();}
