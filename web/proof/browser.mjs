import { trustAuthority } from "./trusted-authority.mjs";
// Browser/display lifetime and observed rendering capability for local proofs.
import { spawn, execFileSync } from 'node:child_process';
import { accessSync, constants, readdirSync } from 'node:fs';
import { access, mkdtemp, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';

export class ProofUnavailable extends Error { exitCode = 3; }

export function rendererKind(renderer) {
  if (!renderer || /generic|or similar/i.test(renderer)) return 'unknown';
  if (/swiftshader|llvmpipe|softpipe|software|lavapipe|swrast/i.test(renderer)) return 'software';
  if (/intel|nvidia|amd|radeon|apple|adreno|mali|powervr/i.test(renderer)) return 'hardware';
  return 'unknown';
}

function executable(name) {
  try { return execFileSync('which', [name], { encoding: 'utf8' }).trim(); } catch { return null; }
}

function renderDeviceAccessible() {
  try {
    return readdirSync('/dev/dri').filter(name => /^renderD\d+$/.test(name)).some(name => {
      try { accessSync(`/dev/dri/${name}`, constants.R_OK | constants.W_OK); return true; } catch { return false; }
    });
  } catch { return false; }
}

async function stopChild(child) {
  if (child.pid === undefined || child.exitCode !== null || child.signalCode !== null) return;
  let timer;
  try {
    await new Promise(resolve => {
      child.once('exit', resolve);
      child.kill('SIGTERM');
      timer = setTimeout(() => child.kill('SIGKILL'), 2_000);
    });
  } finally { clearTimeout(timer); }
}

async function startDisplay(mode, multipleWindows) {
  const env = { ...process.env };
  if (mode === 'hardware') {
    delete env.LIBGL_ALWAYS_SOFTWARE;
    if (env.WAYLAND_DISPLAY) return { env: { ...env, MOZ_ENABLE_WAYLAND: '1' }, stop: async () => {} };
    if (env.DISPLAY) return { env, stop: async () => {} };
    const weston = executable('weston');
    if (!weston) throw new ProofUnavailable('Hardware Firefox needs a display or Weston; install Weston or select TME_PROOF_RENDERER=software.');
    const runtime = await mkdtemp(path.join(os.tmpdir(), 'tme-proof-wayland-'));
    Object.assign(env, { XDG_RUNTIME_DIR: runtime, WAYLAND_DISPLAY: 'tme-proof', MOZ_ENABLE_WAYLAND: '1' });
    delete env.DISPLAY;
    const child = spawn(weston, ['--backend=headless', '--renderer=gl', '--socket=tme-proof', '--width=1600', '--height=1000', '--idle-time=0', '--no-config', multipleWindows ? '--shell=desktop-shell.so' : '--shell=kiosk-shell.so'], { env, stdio: ['ignore', 'ignore', 'pipe'] });
    let output = '';
    child.stderr.on('data', chunk => { output = (output + chunk).slice(-8_000); });
    let launchError;
    child.on('error', error => { launchError = error; });
    const stop = async () => { try { await stopChild(child); } finally { await rm(runtime, { recursive: true, force: true }); } };
    try {
      const until = Date.now() + 10_000;
      while (Date.now() < until) {
        if (launchError || child.exitCode !== null || child.signalCode !== null) throw new Error(launchError?.message || output);
        try { await access(path.join(runtime, 'tme-proof')); return { env, stop }; } catch { /* not ready */ }
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      throw new Error(`No Wayland socket after 10 seconds. ${output}`);
    } catch (error) { await stop(); throw new ProofUnavailable(`Weston could not start: ${error.message}`); }
  }
  env.LIBGL_ALWAYS_SOFTWARE = '1';
  env.MOZ_ENABLE_WAYLAND = '0';
  delete env.WAYLAND_DISPLAY;
  if (env.DISPLAY) return { env, stop: async () => {} };
  const xvfb = executable('Xvfb');
  if (!xvfb) throw new ProofUnavailable('Software Firefox needs DISPLAY or Xvfb.');
  const child = spawn(xvfb, ['-displayfd', '1', '-screen', '0', '1600x1000x24', '-nolisten', 'tcp'], { stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '', number = '';
  child.stderr.on('data', chunk => { output = (output + chunk).slice(-8_000); });
  child.stdout.on('data', chunk => { number += chunk; });
  let launchError;
  child.on('error', error => { launchError = error; });
  try {
    const until = Date.now() + 10_000;
    while (Date.now() < until) {
      if (launchError || child.exitCode !== null || child.signalCode !== null) throw new Error(launchError?.message || output);
      if (/^\d+\n$/.test(number)) return { env: { ...env, DISPLAY: `:${number.trim()}` }, stop: () => stopChild(child) };
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    throw new Error('No Xvfb display after 10 seconds.');
  } catch (error) { await stopChild(child); throw new ProofUnavailable(`Xvfb could not start: ${error.message}`); }
}

export async function probeRenderer(browser) {
  const page = await browser.newPage();
  try {
    return await page.evaluate(() => {
      const gl = document.createElement('canvas').getContext('webgl2');
      if (gl === null) return null;
      const debug = gl.getExtension('WEBGL_debug_renderer_info');
      const renderer = gl.getParameter(debug ? debug.UNMASKED_RENDERER_WEBGL : gl.RENDERER);
      gl.getExtension('WEBGL_lose_context')?.loseContext();
      return renderer;
    });
  } finally { await page.close(); }
}

export async function launchProofBrowser({ name, engine, executablePath, trustedAuthority, multipleWindows = false }) {
  const requested = process.env.TME_PROOF_RENDERER?.trim() || 'auto';
  if (!['auto', 'hardware', 'software'].includes(requested)) throw new ProofUnavailable(`Unknown TME_PROOF_RENDERER: ${requested}`);
  const linux = process.platform === 'linux';
  const displayAvailable = !!(process.env.WAYLAND_DISPLAY || process.env.DISPLAY || executable('weston'));
  const mode = requested === 'auto'
    ? (!linux || (renderDeviceAccessible() && (name !== 'firefox' || displayAvailable)) ? 'hardware' : 'software')
    : requested;
  let display, browser, trust;
  try {
    if (trustedAuthority) trust = await trustAuthority(name, trustedAuthority);
    if (name === 'chromium') {
      const args = mode === 'software' ? ['--use-angle=swiftshader', '--enable-unsafe-swiftshader']
        : linux ? ['--enable-gpu', '--use-gl=angle', '--use-angle=gl-egl'] : [];
      const env = { ...process.env };
      if (mode === 'hardware') delete env.LIBGL_ALWAYS_SOFTWARE;
      browser = await engine.launch({ executablePath, headless: true, args, env });
    } else {
      display = await startDisplay(mode, multipleWindows);
      const launch = trust?.profile ? options => engine.launchPersistentContext(trust.profile, options) : options => engine.launch(options);
      browser = await launch({ executablePath, headless: false, env: display.env,
        firefoxUserPrefs: { 'webgl.force-enabled': true, 'webgl.forbid-software': mode === 'hardware',
          // Ephemeral proof profiles disclose the actual adapter, not a privacy bucket.
          'webgl.sanitize-unmasked-renderer': false },
      });
    }
    const renderer = await probeRenderer(browser);
    const observed = rendererKind(renderer);
    if (observed !== mode) throw new ProofUnavailable(`${name}: requested ${mode}, observed ${observed} (${renderer ?? 'no WebGL2'}). No renderer substitution is accepted.`);
    console.log(`RENDERER ${name}: ${observed} — ${renderer}`);
    return { browser, context: trust?.profile ? browser : undefined, renderer, rendering: observed,
      stop: async () => { try { await browser.close(); } finally { try { await display?.stop(); } finally { await trust?.stop(); } } },
    };
  } catch (error) {
    try { await browser?.close(); } finally { try { await display?.stop(); } finally { await trust?.stop(); } }
    if (error instanceof ProofUnavailable) throw error;
    throw new ProofUnavailable(`${name} ${mode} browser could not start: ${error.message}`);
  }
}
