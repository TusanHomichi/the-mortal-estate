import { BufferAttribute, BufferGeometry, Color, Mesh, ShaderMaterial, Vector2 } from "three";
import type { FeelSpace } from "../feelTypes";
import { TERRAIN_SUBDIVISIONS, type TerrainSurface } from "../terrainSurface";
import type { Cell } from "../walk/layoutPassability";

/** Cell addresses come from the packet, heights from the same surface as feet. */
export function buildTacticalGridGeometry(space: FeelSpace, surface: TerrainSurface): BufferGeometry {
  const positions: number[] = [], indices: number[] = [];
  const n = surface.coastal ? TERRAIN_SUBDIVISIONS : 1;
  for (const cell of space.cells) {
    // The scenic sea has no standing surface. Shore refusal remains cursor-owned.
    if (cell.material === "water" || cell.material === "void") continue;
    const start = positions.length / 3;
    for (let z = 0; z <= n; z++) for (let x = 0; x <= n; x++) {
      const i = cell.i - .5 + x / n, j = cell.j - .5 + z / n;
      positions.push(i, surface.sample(i, j, cell.material).height + .009, j);
    }
    for (let z = 0; z < n; z++) for (let x = 0; x < n; x++) {
      const a = start + z * (n + 1) + x, b = a + n + 1;
      indices.push(a, b, b + 1, a, b + 1, a + 1);
    }
  }
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new BufferAttribute(new Float32Array(positions), 3));
  geometry.setIndex(indices); geometry.computeBoundingSphere();
  return geometry;
}

/** A quiet two-tone ruled edge, with short corner ticks and local cursor emphasis.
 * It names cells, never reachable range, collision, ownership or line of sight.
 */
export function createTacticalGrid(space: FeelSpace, surface: TerrainSurface) {
  const material = new ShaderMaterial({
    name: "Tactical cell edges", transparent: true, depthWrite: false,
    uniforms: {
      ink: { value: new Color("#28352d") }, chalk: { value: new Color("#c4c6ad") },
      focus: { value: new Vector2() }, focusEnabled: { value: 0 },
    },
    vertexShader: `varying vec2 worldCell;
      void main(){vec4 world=modelMatrix*vec4(position,1.);worldCell=world.xz;
      gl_Position=projectionMatrix*viewMatrix*world;}`,
    fragmentShader: `varying vec2 worldCell;
      uniform vec3 ink;uniform vec3 chalk;uniform vec2 focus;uniform float focusEnabled;
      void main(){
        vec2 f=fract(worldCell+.5);vec2 d=min(f,1.-f);
        vec2 px=max(fwidth(worldCell),vec2(.0001));
        float edge=min(d.x/px.x,d.y/px.y);
        float corner=1.-smoothstep(.07,.16,max(d.x,d.y));
        float nearby=(1.-smoothstep(.7,2.1,length(worldCell-focus)))*focusEnabled;
        float outer=1.-smoothstep(.65,1.8,edge);
        float fine=1.-smoothstep(.1,.65,edge);
        float strength=.19+.13*corner+.2*nearby;
        gl_FragColor=vec4(mix(ink,chalk,fine*.8),outer*strength);
        #include <tonemapping_fragment>
        #include <colorspace_fragment>
      }`,
  });
  const mesh = new Mesh(buildTacticalGridGeometry(space, surface), material);
  mesh.name = "TacticalGrid";
  mesh.renderOrder = 1;
  return {
    mesh,
    focus(cell: Cell | null) {
      material.uniforms.focusEnabled!.value = cell === null ? 0 : 1;
      if (cell) material.uniforms.focus!.value.set(cell.i, cell.j);
    },
  };
}
