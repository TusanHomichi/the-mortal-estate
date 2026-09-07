import { describe, expect, it } from 'vitest';
import { rendererKind, launchProofBrowser, ProofUnavailable } from '../proof/browser.mjs';
import { ENGINE_NAMES, resolveProofBrowsers } from '../proof/engines.mjs';
import { drmCounters } from '../proof/webkit-renderer.mjs';
import { trustSystemAuthority } from '../proof/trusted-authority.mjs';

describe('proof renderer evidence', () => {
  it('refuses software adapters even inside an ANGLE wrapper', () => {
    for (const name of ['ANGLE (Google, Vulkan (SwiftShader Device (Subzero)), SwiftShader driver)', 'llvmpipe (LLVM 20.1, 256 bits)', 'softpipe', 'Mesa lavapipe']) {
      expect(rendererKind(name)).toBe('software');
    }
    expect(rendererKind('ANGLE (Intel, Mesa Intel(R) UHD Graphics 630 (CFL GT2), OpenGL ES 3.2)')).toBe('hardware');
  });
  it('does not accept absent or privacy-sanitized identity as GPU evidence', () => {
    for (const name of [null, '', 'Generic Renderer', 'Intel(R) HD Graphics 400, or similar', 'WebKit WebGL', 'Apple GPU']) {
      expect(rendererKind(name)).toBe('unknown');
    }
  });
  it('rejects an invalid mode before launching a browser', async () => {
    const previous = process.env.TME_PROOF_RENDERER;
    process.env.TME_PROOF_RENDERER = 'wishful-thinking';
    let launched = false;
    try {
      await expect(launchProofBrowser({ name: 'chromium', engine: { launch: () => { launched = true; } } })).rejects.toBeInstanceOf(ProofUnavailable);
      expect(launched).toBe(false);
    } finally {
      if (previous === undefined) delete process.env.TME_PROOF_RENDERER;
      else process.env.TME_PROOF_RENDERER = previous;
    }
  });

  it('closes a browser whose software renderer cannot satisfy a hardware request', async () => {
    const previous = process.env.TME_PROOF_RENDERER;
    process.env.TME_PROOF_RENDERER = 'hardware';
    let browserClosed = false;
    let pageClosed = false;
    const browser = {
      newPage: async () => ({ evaluate: async () => 'ANGLE (Google, SwiftShader)', close: async () => { pageClosed = true; } }),
      close: async () => { browserClosed = true; },
    };
    try {
      await expect(launchProofBrowser({ name: 'chromium', engine: { launch: async () => browser } })).rejects.toThrow('observed software');
      expect(browserClosed).toBe(true);
      expect(pageClosed).toBe(true);
    } finally {
      if (previous === undefined) delete process.env.TME_PROOF_RENDERER;
      else process.env.TME_PROOF_RENDERER = previous;
    }
  });
});

describe('complete browser roster', () => {
  it('includes WebKit and refuses a missing engine rather than narrowing all', () => {
    expect(ENGINE_NAMES).toEqual(['chromium', 'firefox', 'webkit']);
    const engines = Object.fromEntries(ENGINE_NAMES.map(name => [name, { executablePath: () => `/test/${name}` }]));
    expect(resolveProofBrowsers('all', engines, () => true).map(e => e.name)).toEqual(ENGINE_NAMES);
    expect(() => resolveProofBrowsers('all', engines, p => !p.endsWith('webkit'))).toThrow('no webkit');
    expect(resolveProofBrowsers('webkit', engines, () => true).map(e => e.name)).toEqual(['webkit']);
    expect(() => resolveProofBrowsers('safari', engines, () => true)).toThrow('unknown engine');
  });

  it('requires actual driver, client and nanosecond counters for native GPU evidence', () => {
    expect(drmCounters('drm-driver: i915\ndrm-client-id: 42\ndrm-engine-render:\t9876543210987654321 ns\n'))
      .toEqual({ driver: 'i915', client: '42', counters: { render: 9876543210987654321n } });
    expect(drmCounters('drm-driver: i915\ndrm-client-id: 42\n')).toBeNull();
    expect(drmCounters('drm-engine-render: 20 ns\n')).toBeNull();
  });

  it('removes only the uniquely installed WebKit trust anchor on stop', async () => {
    const commands = [];
    const trust = await trustSystemAuthority('/private/test-ca.pem', (command, args) => commands.push([command, args]));
    const anchor = commands[0][1].at(-1);
    expect(anchor).toMatch(/^\/usr\/local\/share\/ca-certificates\/tme-proof-[\da-f-]+\.crt$/);
    await trust.stop(); await trust.stop();
    expect(commands.map(c => c[1][1])).toEqual(['install', 'update-ca-certificates', 'rm', 'update-ca-certificates']);
    expect(commands[2][1]).toEqual(['-n', 'rm', '-f', '--', anchor]);
  });

  it('cleans an installed anchor when updating the trust store fails', async () => {
    const commands = [];
    await expect(trustSystemAuthority('/private/test-ca.pem', (command, args) => {
      commands.push([command, args]);
      if (commands.length === 2) throw new Error('trust update failed');
    })).rejects.toThrow('trust update failed');
    expect(commands.map(c => c[1][1])).toEqual(['install', 'update-ca-certificates', 'rm', 'update-ca-certificates']);
  });

  it('can retry a failed trust-store cleanup after the anchor file was removed', async () => {
    const commands = [];
    const trust = await trustSystemAuthority('/private/test-ca.pem', (command, args) => {
      commands.push([command, args]);
      if (commands.length === 4) throw new Error('cleanup update failed');
    });
    await expect(trust.stop()).rejects.toThrow('cleanup update failed');
    await trust.stop();
    expect(commands).toHaveLength(6);
    expect(commands[4][1][1]).toBe('rm');
    expect(commands[5][1][1]).toBe('update-ca-certificates');
  });
});
