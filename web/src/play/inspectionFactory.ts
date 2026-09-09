import { AuthoritativeRenderer } from "../authoritative/renderer";

/** Explicit fixture diagnostics; never selected by navigation or missing art. */
export async function createPlayRenderer(canvas: HTMLCanvasElement) {
  return new AuthoritativeRenderer(canvas, 768, 512);
}
