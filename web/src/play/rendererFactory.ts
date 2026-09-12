import { WorldRenderer } from './worldRenderer';
/** One playable shell and one 3D renderer for every authored area. */
export function createPlayRenderer(canvas:HTMLCanvasElement){return WorldRenderer.create(canvas);}
