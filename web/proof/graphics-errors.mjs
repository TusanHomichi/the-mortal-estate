/** GPU draw failures are console diagnostics, not JavaScript page errors. */
export function collectGraphicsErrors(page,errors){
 page.on('console',message=>{
  const text=message.text();
  if(/GL_INVALID_|GL_OUT_OF_MEMORY|WebGLProgram: Shader Error/.test(text))errors.push(text);
 });
}
