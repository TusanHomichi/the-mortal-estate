import { CanvasTexture, SRGBColorSpace } from "three";

function traceSole(context: CanvasRenderingContext2D): void {
  context.beginPath();
  context.moveTo(48, 12);
  context.bezierCurveTo(65, 12, 76, 24, 74, 42);
  context.bezierCurveTo(73, 54, 66, 60, 59, 66);
  context.bezierCurveTo(54, 72, 55, 82, 61, 94);
  context.bezierCurveTo(67, 108, 63, 127, 51, 132);
  context.bezierCurveTo(38, 137, 27, 127, 28, 113);
  context.bezierCurveTo(29, 101, 36, 92, 36, 81);
  context.bezierCurveTo(36, 72, 29, 66, 24, 57);
  context.bezierCurveTo(14, 40, 21, 20, 38, 14);
  context.bezierCurveTo(41, 13, 45, 12, 48, 12);
  context.closePath();
}

function paintSoleLayer(
  context: CanvasRenderingContext2D,
  colour: string,
  opacity: number,
  blur: number,
): void {
  context.save();
  // Canvas bottom becomes the print's toe after the ground-plane rotation.
  context.translate(16, 168);
  context.scale(1, -1);
  context.filter = `blur(${blur}px)`;
  context.globalAlpha = opacity;
  context.fillStyle = colour;
  traceSole(context);
  context.fill();
  context.restore();
}

export function makeSoleTexture(kind: "draft" | "committed"): CanvasTexture {
  const drawing = document.createElement("canvas");
  drawing.width = 128;
  drawing.height = 192;
  const context = drawing.getContext("2d");
  if (context === null) throw new Error("the walk experiment could not draw its footprints");
  context.clearRect(0, 0, drawing.width, drawing.height);
  paintSoleLayer(context, "#8fb4ff", kind === "draft" ? 0.48 : 0.68, kind === "draft" ? 8 : 11);
  paintSoleLayer(context, "#dfeaff", 0.9, 2.4);
  paintSoleLayer(context, "#dfeaff", 0.55, 0.8);
  const texture = new CanvasTexture(drawing);
  texture.colorSpace = SRGBColorSpace;
  texture.needsUpdate = true;
  return texture;
}
