import { describe, expect, it } from "vitest";
import {
  WALK_CURSOR_HOTSPOT,
  WALK_CURSOR_SIZE,
  walkCursorDataUris,
} from "../src/walk/cursors";

function decodeSvg(dataUri: string): string {
  const prefix = "data:image/svg+xml,";
  expect(dataUri.startsWith(prefix)).toBe(true);
  return decodeURIComponent(dataUri.slice(prefix.length));
}

function tagStack(svg: string): string[] {
  const stack: string[] = [];
  for (const match of svg.matchAll(/<([^>]+)>/g)) {
    const tag = match[1]!.trim();
    if (tag.startsWith("?") || tag.startsWith("!")) continue;
    if (tag.startsWith("/")) {
      expect(stack.pop()).toBe(tag.slice(1));
      continue;
    }
    const name = /^[A-Za-z][\w:-]*/.exec(tag)?.[0];
    expect(name).toBeDefined();
    if (!tag.endsWith("/")) stack.push(name!);
  }
  return stack;
}

describe("walk cursor artwork", () => {
  it("returns three well-formed SVG data URIs at the declared 24-pixel size", () => {
    const cursors = walkCursorDataUris();
    expect(Object.keys(cursors).sort()).toEqual(["ready", "refused", "waiting"]);

    for (const dataUri of Object.values(cursors)) {
      const svg = decodeSvg(dataUri);
      expect(svg).toMatch(/^<svg\b[^>]*>[\s\S]*<\/svg>$/);
      expect(svg).toContain(`width="${WALK_CURSOR_SIZE}"`);
      expect(svg).toContain(`height="${WALK_CURSOR_SIZE}"`);
      expect(svg).toContain(`viewBox="0 0 ${WALK_CURSOR_SIZE} ${WALK_CURSOR_SIZE}"`);
      expect(tagStack(svg)).toEqual([]);
      expect(new Set(svg.match(/#[0-9a-f]{6}/gi))).toEqual(
        new Set(["#eef2f3", "#17232b"]),
      );
    }
  });

  it("puts the CSS hotspot at the arrow tip", () => {
    expect(WALK_CURSOR_HOTSPOT).toEqual({ x: 2, y: 2 });
  });
});
