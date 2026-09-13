/** Snapshot of the verified 3D menu scene; no credentials or live actor state. */
export function prepareEntryBackdrop(source: HTMLCanvasElement): () => void {
  const canvas=document.createElement('canvas');canvas.id='entry-scene';canvas.setAttribute('aria-hidden','true');
  canvas.width=source.width;canvas.height=source.height;
  const context=canvas.getContext('2d');if(!context)throw Error('Entry artwork could not be prepared.');
  context.drawImage(source,0,0);document.getElementById('entry')!.prepend(canvas);
  const resize=()=>{const scale=Math.max(innerWidth/canvas.width,innerHeight/canvas.height);
    canvas.style.width=`${canvas.width*scale}px`;canvas.style.height=`${canvas.height*scale}px`;};
  resize();window.addEventListener('resize',resize);
  return ()=>{window.removeEventListener('resize',resize);canvas.remove();};
}
