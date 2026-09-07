// WebKit masks even UNMASKED_RENDERER_WEBGL. On Linux, prove GPU execution
// in this launch's WebKit content process, never in the display compositor.
import { readdir, readFile } from "node:fs/promises";

export function drmCounters(text) {
  const driver = /^drm-driver:\s*(\S+)/m.exec(text)?.[1];
  const client = /^drm-client-id:\s*(\d+)/m.exec(text)?.[1];
  const counters = [...text.matchAll(/^drm-engine-(\S+):\s*(\d+) ns$/gm)];
  if (!driver || !client || !counters.length) return null;
  return { driver, client, counters: Object.fromEntries(counters.map(([, name, ns]) => [name, BigInt(ns)])) };
}

async function snapshot(token) {
  const samples = new Map();
  for (const pid of (await readdir("/proc")).filter(name => /^\d+$/.test(name))) {
    try {
      const command = await readFile(`/proc/${pid}/comm`, "utf8");
      if (!/^WebKit(Web|GPU)/.test(command)) continue;
      const env = await readFile(`/proc/${pid}/environ`, "utf8");
      if (!env.split("\0").includes(`TME_WEBKIT_PROBE=${token}`)) continue;
      for (const fd of await readdir(`/proc/${pid}/fdinfo`)) {
        const row = drmCounters(await readFile(`/proc/${pid}/fdinfo/${fd}`, "utf8"));
        if (row) samples.set(`${pid}:${row.driver}:${row.client}`, row);
      }
    } catch { /* Processes/fds may exit during enumeration; missing proof refuses. */ }
  }
  return samples;
}

export async function probeWebKitHardware(browser, token) {
  const page = await browser.newPage();
  try {
    const viewport = page.viewportSize();
    const size = await page.evaluate(() => ({ width: innerWidth, height: innerHeight, scale: devicePixelRatio }));
    if (size.width !== viewport.width || size.height !== viewport.height || size.scale !== 1) {
      throw new Error(`WebKit proof viewport differs from requested CSS pixels: ${JSON.stringify(size)}`);
    }
    await page.evaluate(() => {
      const canvas = document.createElement("canvas"); canvas.width = canvas.height = 256;
      const gl = canvas.getContext("webgl2");
      if (!gl) throw new Error("WebKit has no WebGL2 context");
      window.probeGL = gl;
    });
    const before = await snapshot(token);
    await page.evaluate(() => {
      const gl = window.probeGL;
      const program = gl.createProgram();
      for (const [type, source] of [[gl.VERTEX_SHADER, '#version 300 es\nvoid main(){vec2 p=vec2(float((gl_VertexID<<1)&2),float(gl_VertexID&2));gl_Position=vec4(p*2.-1.,0.,1.);}'],
        [gl.FRAGMENT_SHADER, '#version 300 es\nprecision highp float;out vec4 c;void main(){c=vec4(.25,.5,.75,1.);}']]) {
        const shader = gl.createShader(type); gl.shaderSource(shader, source); gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
        gl.attachShader(program, shader);
      }
      gl.linkProgram(program); gl.useProgram(program);
      const pixel = new Uint8Array(4);
      for (let i = 0; i < 64; i++) {
        gl.drawArrays(gl.TRIANGLES, 0, 3);
        gl.readPixels(128, 128, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
      }
      if (Math.abs(pixel[1] - 128) > 1 || gl.getError() !== gl.NO_ERROR) throw new Error("WebKit GPU probe pixel failed");
      gl.finish();
    });
    const after = await snapshot(token);
    for (const [key, row] of after) {
      const earlier = before.get(key);
      if (!earlier) continue;
      for (const [engine, ns] of Object.entries(row.counters)) {
        if (ns > (earlier.counters[engine] ?? ns)) return `WebKitGTK WebGL2 — DRM ${row.driver} ${engine}, GPU execution observed`;
      }
    }
    throw new Error("WebKit masks its adapter; no launch-owned DRM execution was observed");
  } finally { await page.close(); }
}
