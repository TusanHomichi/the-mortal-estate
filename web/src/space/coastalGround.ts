import { lightingPeriod } from "../presets";
import { addCoastalWater } from "./coastalWater";
import { BufferAttribute, BufferGeometry, Color, Group, Mesh, ShaderMaterial, ShadowMaterial } from "three";
import { SEA_HEIGHT, TERRAIN_SUBDIVISIONS, type TerrainSurface } from "../terrainSurface";
import { groundFragmentShader, groundVertexShader } from "../shaders";
import type { SpaceSceneOptions } from "./SpaceScene";
import { keyLightOffset, type ScenePalette } from "./palette";
import { configureTexture, requiredTexture } from "./textures";
import { pathCover } from "./groundCover";
import { addGroundStones } from "./groundStones";

const vertexShader = groundVertexShader.replace("attribute vec2 cellOrigin;", "attribute vec2 cellOrigin; attribute vec3 cover; varying vec3 vCover;")
  .replace("vUv = uv;", "vUv = uv; vCover = cover;");
const fragmentShader = groundFragmentShader.replace("uniform sampler2D swatch;", `
  uniform sampler2D swatch; uniform sampler2D laneSwatch; uniform sampler2D meadowSwatch;
  varying vec3 vCover; uniform float seaHeight;
  float groundNoise(vec2 p) {
    vec2 i=floor(p), f=fract(p); f=f*f*(3.0-2.0*f);
    vec4 seed=vec4(dot(i,vec2(127.1,311.7)),dot(i+vec2(1,0),vec2(127.1,311.7)),
      dot(i+vec2(0,1),vec2(127.1,311.7)),dot(i+vec2(1,1),vec2(127.1,311.7)));
    vec4 n=fract(sin(seed)*43758.5453);
    return mix(mix(n.x,n.y,f.x),mix(n.z,n.w,f.x),f.y);
  }
`).replace("vec3 base = texture2D(swatch, worldUv).rgb * timeTint;", `
  // The continuous seabed owns submerged natural ground.
  if (vWorldPosition.y < seaHeight) discard;
  vec3 turf = mix(texture2D(swatch, worldUv).rgb, texture2D(meadowSwatch, worldUv).rgb, smoothstep(.2, .8, vCover.z)) * timeTint;
  vec3 path = texture2D(laneSwatch, worldUv).rgb * vec3(.79, .75, .65);
  float pathMix = smoothstep(.27, .70, vCover.y);
  vec3 base = mix(turf, path, pathMix);
  float broadWear = sin(vWorldPosition.x * .81 + sin(vWorldPosition.z * .67))
    * sin(vWorldPosition.z * 1.09 - vWorldPosition.x * .24);
  base *= .89 + .10 * broadWear + .15 * groundNoise(vWorldPosition.xz * 3.7);
  float scuff = smoothstep(.61,.79,groundNoise(vWorldPosition.xz * 13.0)) * pathMix;
  base = mix(base, path * vec3(.62,.59,.51), scuff * .26);
  // Quiet patches of dry turf and small worn stones break up the path field.
  // The logical joints are drawn later and retain their gameplay presentation.
  float dryTurf = smoothstep(.12, .75, broadWear) * (1.0 - pathMix) * .24;
  base = mix(base, path * vec3(.68, .74, .53), dryTurf);
  vec2 pebbleCell = floor(vWorldPosition.xz * 5.0);
  float pebbleSeed = fract(sin(dot(pebbleCell, vec2(127.1, 311.7))) * 43758.5453);
  vec2 pebbleOffset = fract(vWorldPosition.xz * 5.0) - vec2(.25 + pebbleSeed * .45, .5);
  float pebble = (1.0 - smoothstep(.12, .22, length(pebbleOffset * vec2(1.0, 1.4))))
    * step(.74, pebbleSeed) * pathMix;
  base = mix(base, path * vec3(1.06, 1.04, .97), pebble * .65);
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
    keyDirection: { value: keyLightOffset(presets, space.weather).normalize() },
  };
  const pathName = space.cells.some(c => c.material === "lane") ? "lane" : "earth";
  const meadowName = space.cells.some(c => c.material === "meadow") ? "meadow" : "grass";
  const material = new ShaderMaterial({ name: "CoastalGround", vertexShader, fragmentShader,
    uniforms: { ...shared, seaHeight: { value: SEA_HEIGHT }, swatch: { value: texture("grass") }, laneSwatch: { value: texture(pathName) }, meadowSwatch: { value: texture(meadowName) },
      swatchPeriod: { value: 3 }, waterSurface: { value: 0 }, timeTint: { value: new Color(.82, .85, .78) } } });
  // Only named natural surfaces participate in the coast blend. Every other
  // authored material retains its own verified swatch and contact geometry.
  const buckets = new Map<string, typeof space.cells>();
  for (const cell of space.cells) {
    // Rounded pond banks can occupy corners of logical water cells. The water
    // datum clips their submerged fragments; cell legality is unchanged.
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
  const shadowMaterial = new ShadowMaterial({ color: 0x02050b, opacity: lightingPeriod(presets) === "day" ? .46 : .34 });
  // This overlay belongs to the exposed ground. Letting its finer submerged
  // triangles overlap the separate seabed produces alternating shadow patches.
  shadowMaterial.onBeforeCompile = shader => {
    shader.uniforms.groundSeaHeight = { value: SEA_HEIGHT };
    shader.vertexShader = "varying float vGroundHeight;\n" + shader.vertexShader.replace("#include <begin_vertex>",
      "#include <begin_vertex>\nvGroundHeight=(modelMatrix*vec4(transformed,1.0)).y;");
    shader.fragmentShader = "varying float vGroundHeight; uniform float groundSeaHeight;\n" + shader.fragmentShader.replace("void main() {",
      "void main() {\nif(vGroundHeight < groundSeaHeight) discard;");
  };
  const shadows = new Mesh(geometry, shadowMaterial);
  shadows.renderOrder = -2; // Bank shadows blend before the transparent sea.
  shadows.name = "GroundShadowReceiver"; shadows.position.y = .003; shadows.receiveShadow = true; group.add(shadows);
  }
  addCoastalWater(group, space, surface, elapsed, palette, options.camera, keyLightOffset(presets, space.weather));
  addGroundStones(group, space, surface);
}
