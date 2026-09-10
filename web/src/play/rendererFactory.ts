import { WorldRenderer } from './worldRenderer';
/** One playable shell; authored dungeon areas select their 3D presentation. */
export function createPlayRenderer(canvas:HTMLCanvasElement){return WorldRenderer.create(canvas);}
