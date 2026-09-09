import { PixelRenderer } from "./pixelRenderer";

/** The playable product has one renderer. Inspection builds replace this module. */
export function createPlayRenderer(canvas: HTMLCanvasElement) {
  return PixelRenderer.create(canvas, 768, 768);
}
