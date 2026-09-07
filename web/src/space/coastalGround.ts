import { addCoastalWater } from "./coastalWater";
import { BufferAttribute, BufferGeometry, Color, Group, Mesh, ShaderMaterial, ShadowMaterial, Vector3 } from "three";
import { TERRAIN_SUBDIVISIONS, type TerrainSurface } from "../terrainSurface";
import { groundFragmentShader, groundVertexShader } from "../shaders";
import type { SpaceSceneOptions } from "./SpaceScene";
import { keyLightOffset, type ScenePalette } from "./palette";
import { configureTexture, requiredTexture } from "./textures";
import { pathCover } from "./groundCover";

const vertexShader = groundVertexShader.replace("attribute vec2 cellOrigin;", "attribute vec2 cellOrigin; attribute vec3 cover; varying vec3 vCover;")
  .replace("vUv = uv;", "vUv = uv; vCover = cover;");
const fragmentShader = groundFragmentShader.replace("uniform sampler2D swatch;", `
  uniform sampler2D swatch; uniform sampler2D laneSwatch; uniform sampler2D meadowSwatch;
  varying vec3 vCover;
`).replace("vec3 base = texture2D(swatch, worldUv).rgb * timeTint;", `
  vec3 turf = mix(texture2D(swatch, worldUv).rgb, texture2D(meadowSwatch, worldUv).rgb, smoothstep(.2, .8, vCover.z)) * timeTint;
  vec3 path = texture2D(laneSwatch, worldUv).rgb * vec3(.79, .75, .65);
  float pathMix = smoothstep(.27, .70, vCover.y);
  vec3 base = mix(turf, path, pathMix);
  float broadWear = sin(vWorldPosition.x * .81 + sin(vWorldPosition.z * .67))
    * sin(vWorldPosition.z * 1.09 - vWorldPosition.x * .24);
  base *= .94 + .06 * broadWear;
  float bankMix = 1.0 - smoothstep(.52, .86, vCover.x);
  vec3 bank = path * vec3(.57, .51, .42);
  base = mix(base, bank, bankMix);
`);

/** A continuous bank and path surface above a separate, lower water plane. */
export function addCoastalGround(group: Group, options: SpaceSceneOptions, palette: ScenePalette,
  elapsed: { value: number }, surface: TerrainSurface): void {
  const { space, textures, presets, anisotropy } = options;
  const texture = (name: string) => {
    const result = requiredTexture(textures, `terrain/${name}`).texture;
    configureTexture(result, anisotropy); return result;
  };
  const shared = {
    elapsed, wetness: { value: presets.includes("rain") ? 1 : 0 },
    ambientColour: { value: palette.ambient.clone().multiplyScalar(palette.ambientIntensity) },
    keyColour: { value: palette.key.clone().multiplyScalar(palette.keyIntensity * .44) },
    keyDirection: { value: new Vector3(-.52, .79, -.33).normalize() },
  };
  const pathName = space.cells.some(c => c.material === "lane") ? "lane" : "earth";
  const meadowName = space.cells.some(c => c.material === "meadow") ? "meadow" : "grass";
  const material = new ShaderMaterial({ name: "CoastalGround", vertexShader, fragmentShader,
    uniforms: { ...shared, swatch: { value: texture("grass") }, laneSwatch: { value: texture(pathName) }, meadowSwatch: { value: texture(meadowName) },
      swatchPeriod: { value: 3 }, waterSurface: { value: 0 }, timeTint: { value: new Color(.82, .85, .78) } } });
  // Only named natural surfaces participate in the coast blend. Every other
  // authored material retains its own verified swatch and contact geometry.
  const buckets = new Map<string, typeof space.cells>();
  for (const cell of space.cells) {
    if (cell.material === "void") continue;
    const name = ["grass", "meadow", "lane", "water"].includes(cell.material) ? "coast" : cell.material;
    const bucket = buckets.get(name) ?? []; bucket.push(cell); buckets.set(name, bucket);
  }
  for (const [name, cells] of buckets) {
  const p: number[] = [], uv: number[] = [], origin: number[] = [], cover: number[] = [], indices: number[] = [];
  const n = TERRAIN_SUBDIVISIONS;
  for (const cell of cells) {
    const start = p.length / 3;
    for (let z = 0; z <= n; z++) for (let x = 0; x <= n; x++) {
      const i = cell.i - .5 + x / n, j = cell.j - .5 + z / n;
      const s = surface.sample(i, j, cell.material);
      p.push(i, s.height - .006, j); uv.push(x / n, z / n); origin.push(cell.i, cell.j);
      cover.push(s.land, pathCover(s, i, j), s.meadow);
    }
    for (let z = 0; z < n; z++) for (let x = 0; x < n; x++) {
      const a = start + z * (n + 1) + x, b = a + n + 1;
      indices.push(a, b, b + 1, a, b + 1, a + 1);
    }
  }
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new BufferAttribute(new Float32Array(p), 3));
  geometry.setAttribute("uv", new BufferAttribute(new Float32Array(uv), 2));
  geometry.setAttribute("cellOrigin", new BufferAttribute(new Float32Array(origin), 2));
  geometry.setAttribute("cover", new BufferAttribute(new Float32Array(cover), 3));
  geometry.setIndex(indices); geometry.computeVertexNormals(); geometry.computeBoundingSphere();
  const groundMaterial = name === "coast" ? material : new ShaderMaterial({
    name: `ground-${name}`, vertexShader: groundVertexShader, fragmentShader: groundFragmentShader,
    uniforms: { ...shared, swatch: { value: texture(name) }, swatchPeriod: { value: 3 },
      waterSurface: { value: 0 }, timeTint: { value: new Color(.82, .85, .78) } },
  });
  const ground = new Mesh(geometry, groundMaterial); ground.name = `Ground_${name}`; group.add(ground);
  const shadows = new Mesh(geometry, new ShadowMaterial({ color: 0x02050b, opacity: .34 }));
  shadows.name = "GroundShadowReceiver"; shadows.position.y = .003; shadows.receiveShadow = true; group.add(shadows);
  }
  addCoastalWater(group, space, surface, elapsed, palette, options.camera, keyLightOffset(presets, space.weather));
}
