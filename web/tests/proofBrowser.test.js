import { describe, expect, it } from 'vitest';
import { rendererKind, launchProofBrowser, ProofUnavailable } from '../proof/browser.mjs';

describe('proof renderer evidence', () => {
  it('refuses software adapters even inside an ANGLE wrapper', () => {
    for (const name of ['ANGLE (Google, Vulkan (SwiftShader Device (Subzero)), SwiftShader driver)', 'llvmpipe (LLVM 20.1, 256 bits)', 'softpipe', 'Mesa lavapipe']) {
      expect(rendererKind(name)).toBe('software');
    }
    expect(rendererKind('ANGLE (Intel, Mesa Intel(R) UHD Graphics 630 (CFL GT2), OpenGL ES 3.2)')).toBe('hardware');
  });
  it('does not accept absent or privacy-sanitized identity as GPU evidence', () => {
    for (const name of [null, '', 'Generic Renderer', 'Intel(R) HD Graphics 400, or similar', 'WebKit WebGL']) {
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
