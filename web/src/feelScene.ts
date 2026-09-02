import {
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  Clock,
  Color,
  DirectionalLight,
  DoubleSide,
  DynamicDrawUsage,
  InstancedMesh,
  LinearFilter,
  LinearMipmapLinearFilter,
  Matrix4,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  NoToneMapping,
  Object3D,
  OrthographicCamera,
  PlaneGeometry,
  PointLight,
  RepeatWrapping,
  Scene,
  ShaderMaterial,
  ShadowMaterial,
  SRGBColorSpace,
  Texture,
  Vector2,
  Vector3,
  Vector4,
  WebGLRenderer,
} from "three";
import { createFeelCamera, resizeFeelCamera } from "./camera";
import type { FeelManifest, VerifiedAssetPacket } from "./feelTypes";
import type { Preset } from "./presets";
import {
  fogFragmentShader,
  fogVertexShader,
  groundFragmentShader,
  groundVertexShader,
  swayFragmentShader,
  swayVertexShader,
} from "./shaders";
import { buildWallProfile, type GeometryData, type WallMaterial } from "./wallGeometry";
import { createWalkPresenter } from "./walk/walkPresenter";

interface DecodedTexture {
  texture: Texture;
  width: number;
  height: number;
}

interface ScenePalette {
  background: Color;
  ambient: Color;
  ambientIntensity: number;
  key: Color;
  keyIntensity: number;
  lanternIntensity: number;
  candleIntensity: number;
  practicalShaderStrength: number;
}

interface RainSystem {
  mesh: InstancedMesh;
  update: (elapsed: number) => void;
}

export interface FeelSceneHandle {
  renderer: WebGLRenderer;
  camera: OrthographicCamera;
  stop: () => void;
}

const WARM_LIGHT = new Color("#ffb457");
const RAIN_COUNT = 1080;

function paletteFor(presets: readonly Preset[]): ScenePalette {
  return presets.includes("dusk")
    ? {
        background: new Color("#4b394d"),
        ambient: new Color("#d2ddf0"),
        ambientIntensity: 1.2,
        key: new Color("#c5d9ff"),
        keyIntensity: 1.5,
        lanternIntensity: 65,
        candleIntensity: 7,
        practicalShaderStrength: 5,
      }
    : {
        background: new Color("#091426"),
        ambient: new Color("#f2f7ff"),
        ambientIntensity: 1.2,
        key: new Color("#a9caff"),
        keyIntensity: 1,
        lanternIntensity: 55,
        candleIntensity: 5,
        practicalShaderStrength: 4,
      };
}

async function decodeTextures(packet: VerifiedAssetPacket): Promise<Map<string, DecodedTexture>> {
  const decoded = new Map<string, DecodedTexture>();
  await Promise.all(
    [...packet.assets.entries()].map(async ([key, asset]) => {
      // ImageBitmap uploads ignore Texture.flipY in WebGL. Flip while decoding
      // so Three receives the same orientation as its ordinary image loader.
      const bitmap = await createImageBitmap(new Blob([asset.bytes], { type: "image/png" }), {
        imageOrientation: "flipY",
      });
      const texture = new Texture(bitmap);
      texture.name = key;
      texture.colorSpace = SRGBColorSpace;
      texture.wrapS = RepeatWrapping;
      texture.wrapT = RepeatWrapping;
      texture.magFilter = LinearFilter;
      texture.minFilter = LinearMipmapLinearFilter;
      texture.generateMipmaps = true;
      texture.needsUpdate = true;
      decoded.set(key, { texture, width: bitmap.width, height: bitmap.height });
    }),
  );
  return decoded;
}

function requiredTexture(textures: Map<string, DecodedTexture>, key: string): DecodedTexture {
  const texture = textures.get(key);
  if (texture === undefined) throw new Error(`verified texture ${key} was not decoded`);
  return texture;
}

function geometryFromData(data: GeometryData): BufferGeometry {
  const geometry = new BufferGeometry();
  geometry.setAttribute("position", new BufferAttribute(new Float32Array(data.positions), 3));
  geometry.setAttribute("uv", new BufferAttribute(new Float32Array(data.uvs), 2));
  geometry.setIndex(data.indices);
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}

function configureTexture(texture: Texture, anisotropy: number): void {
  texture.anisotropy = anisotropy;
  texture.needsUpdate = true;
}

function addGround(
  scene: Scene,
  manifest: FeelManifest,
  textures: Map<string, DecodedTexture>,
  presets: readonly Preset[],
  palette: ScenePalette,
  anisotropy: number,
): void {
  const rainy = presets.includes("rain");
  const geometry = new PlaneGeometry(1, 1);
  geometry.rotateX(-Math.PI / 2);
  const ambient = palette.ambient.clone().multiplyScalar(palette.ambientIntensity);
  const key = palette.key.clone().multiplyScalar(palette.keyIntensity * 0.44);
  for (const cell of manifest.layout.cells) {
    const swatch = requiredTexture(textures, `terrain/${cell.material}`).texture;
    configureTexture(swatch, anisotropy);
    const material = new ShaderMaterial({
      name: `ground-${cell.material}`,
      uniforms: {
        swatch: { value: swatch },
        cellOrigin: { value: new Vector2(cell.i, cell.j) },
        swatchPeriod: { value: 3 },
        jointWidth: { value: 0.028 },
        wetness: { value: rainy ? 1 : 0 },
        timeTint: {
          value: presets.includes("dusk")
            ? new Color(0.94, 0.94, 0.94)
            : cell.material === "grass"
              ? new Color(0.6, 0.72, 0.9)
              : new Color(0.74, 0.82, 0.96),
        },
        ambientColour: { value: ambient },
        keyColour: { value: key },
        keyDirection: { value: new Vector3(-0.52, 0.79, -0.33).normalize() },
      },
      vertexShader: groundVertexShader,
      fragmentShader: groundFragmentShader,
    });
    const mesh = new Mesh(geometry, material);
    mesh.name = `Cell_${cell.i}_${cell.j}`;
    mesh.position.set(cell.i, -0.006, cell.j);
    mesh.receiveShadow = false;
    scene.add(mesh);
  }

  const extents = manifest.layout.grid_extents;
  const shadowPlane = new Mesh(
    new PlaneGeometry(extents.i, extents.j),
    new ShadowMaterial({ color: 0x02050b, opacity: 0.34 }),
  );
  shadowPlane.name = "GroundShadowReceiver";
  shadowPlane.rotation.x = -Math.PI / 2;
  shadowPlane.position.set((extents.i - 1) / 2, -0.003, (extents.j - 1) / 2);
  shadowPlane.receiveShadow = true;
  scene.add(shadowPlane);
}

function wallMaterials(
  textures: Map<string, DecodedTexture>,
  anisotropy: number,
): Record<WallMaterial, MeshStandardMaterial> {
  const build = (name: WallMaterial, alpha = false): MeshStandardMaterial => {
    const map = requiredTexture(textures, `walls/${name}`).texture;
    configureTexture(map, anisotropy);
    return new MeshStandardMaterial({
      name: `wall-${name}`,
      map,
      roughness: 0.86,
      metalness: 0,
      alphaTest: alpha ? 0.12 : 0,
      transparent: alpha,
      side: alpha ? DoubleSide : undefined,
    });
  };
  return {
    plinth: build("plinth"),
    plaster: build("plaster"),
    sill: build("sill"),
    post: build("post"),
    door: build("door", true),
    cap_front: build("cap_front"),
    cap_top: build("cap_top"),
  };
}

function addWalls(
  scene: Scene,
  manifest: FeelManifest,
  textures: Map<string, DecodedTexture>,
  anisotropy: number,
): void {
  const materials = wallMaterials(textures, anisotropy);
  for (const part of buildWallProfile(manifest.layout.wall_runs)) {
    const mesh = new Mesh(geometryFromData(part.geometry), materials[part.material]);
    mesh.name = part.label;
    mesh.castShadow = true;
    mesh.receiveShadow = true;
    scene.add(mesh);
  }
}

function addContactShadow(scene: Scene, x: number, z: number, height: number): Mesh {
  const shadow = new Mesh(
    new PlaneGeometry(
      Math.min(Math.max(height * 0.34, 0.24), 0.72),
      Math.min(Math.max(height * 0.13, 0.1), 0.28),
    ),
    new MeshBasicMaterial({
      color: 0x000000,
      opacity: 0.46,
      transparent: true,
      depthWrite: false,
    }),
  );
  shadow.name = "ContactShadow";
  shadow.rotation.x = -Math.PI / 2;
  shadow.position.set(x, 0.004, z);
  scene.add(shadow);
  return shadow;
}

function addProps(
  scene: Scene,
  manifest: FeelManifest,
  textures: Map<string, DecodedTexture>,
  presets: readonly Preset[],
  palette: ScenePalette,
  anisotropy: number,
  lanternPosition: Vector3,
): {
  billboards: Mesh[];
  swayMaterials: ShaderMaterial[];
  caretaker: { card: Mesh; contactShadow: Mesh };
} {
  const billboards: Mesh[] = [];
  const swayMaterials: ShaderMaterial[] = [];
  let caretaker: { card: Mesh; contactShadow: Mesh } | null = null;
  for (const prop of manifest.layout.props) {
    const source = requiredTexture(textures, `props/${prop.kind}`);
    configureTexture(source.texture, anisotropy);
    const width = prop.nominal_height * (source.width / source.height);
    const geometry = new PlaneGeometry(width, prop.nominal_height);
    const material = prop.sway
      ? new ShaderMaterial({
          name: `sway-${prop.kind}`,
          uniforms: {
            albedoTexture: { value: source.texture },
            elapsed: { value: 0 },
            windStrength: { value: presets.includes("wind") ? 1 : 0.12 },
            timeOffset: { value: prop.cell_anchor[0] * 0.73 + prop.cell_anchor[1] * 1.13 },
            ambientColour: {
              value: palette.ambient.clone().multiplyScalar(palette.ambientIntensity),
            },
            keyColour: { value: palette.key.clone().multiplyScalar(palette.keyIntensity * 0.34) },
            lanternPosition: { value: lanternPosition },
            lanternColour: { value: WARM_LIGHT.clone() },
            lanternStrength: { value: palette.practicalShaderStrength },
          },
          vertexShader: swayVertexShader,
          fragmentShader: swayFragmentShader,
          transparent: true,
          alphaTest: 0.12,
          side: DoubleSide,
        })
      : new MeshStandardMaterial({
          name: `prop-${prop.kind}`,
          map: source.texture,
          transparent: true,
          alphaTest: 0.12,
          roughness: 0.88,
          metalness: 0,
          side: DoubleSide,
        });
    if (material instanceof ShaderMaterial) swayMaterials.push(material);
    const mesh = new Mesh(geometry, material);
    mesh.name = `Prop_${prop.kind}`;
    mesh.position.set(prop.cell_anchor[0], prop.nominal_height / 2, prop.cell_anchor[1]);
    mesh.castShadow = true;
    mesh.customDepthMaterial = undefined;
    const contactShadow = addContactShadow(
      scene,
      prop.cell_anchor[0],
      prop.cell_anchor[1],
      prop.nominal_height,
    );
    scene.add(mesh);
    billboards.push(mesh);
    if (prop.kind === "caretaker") caretaker = { card: mesh, contactShadow };
  }
  if (caretaker === null) throw new Error("the feel layout carries no caretaker for the walk experiment");
  return { billboards, swayMaterials, caretaker };
}

function addLights(
  scene: Scene,
  manifest: FeelManifest,
  presets: readonly Preset[],
  palette: ScenePalette,
): { lantern: PointLight; lanternBase: number } {
  scene.add(new AmbientLight(palette.ambient, palette.ambientIntensity));
  const key = new DirectionalLight(palette.key, palette.keyIntensity);
  key.name = presets.includes("dusk") ? "WarmHorizonKey" : "CoolMoonlight";
  key.position.set(
    presets.includes("dusk") ? -6 : 8,
    presets.includes("dusk") ? 6 : 12,
    presets.includes("dusk") ? 10 : -7,
  );
  key.target.position.set(4.5, 0, 3.5);
  key.castShadow = true;
  key.shadow.mapSize.set(2048, 2048);
  key.shadow.camera.left = -10;
  key.shadow.camera.right = 10;
  key.shadow.camera.top = 10;
  key.shadow.camera.bottom = -10;
  key.shadow.camera.near = 0.1;
  key.shadow.camera.far = 40;
  key.shadow.bias = -0.00025;
  scene.add(key, key.target);

  const lanternBase = palette.lanternIntensity;
  const lantern = new PointLight(WARM_LIGHT, lanternBase, 6, 2);
  lantern.name = "LanternGlow";
  lantern.position.fromArray(manifest.layout.light_sources.lantern_glass);
  lantern.castShadow = true;
  lantern.shadow.mapSize.set(512, 512);
  scene.add(lantern);
  manifest.layout.light_sources.candles.forEach((position, index) => {
    const candle = new PointLight(WARM_LIGHT, palette.candleIntensity, 2.2, 2);
    candle.name = `Candle_${index}`;
    candle.position.fromArray(position);
    scene.add(candle);
  });
  return { lantern, lanternBase };
}

function seededRandom(seed = 0x544d455f): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

function addRain(scene: Scene, camera: OrthographicCamera): RainSystem {
  const geometry = new PlaneGeometry(0.012, 0.1);
  const material = new MeshBasicMaterial({
    color: new Color(0.65, 0.77, 0.94),
    opacity: 0.42,
    transparent: true,
    depthWrite: false,
    side: DoubleSide,
  });
  const mesh = new InstancedMesh(geometry, material, RAIN_COUNT);
  mesh.name = "RainStreaks";
  mesh.instanceMatrix.setUsage(DynamicDrawUsage);
  mesh.frustumCulled = false;
  const random = seededRandom();
  const drops = Array.from({ length: RAIN_COUNT }, () => ({
    x: -3 + random() * 14,
    z: -3 + random() * 14,
    phase: random() * 6.5,
    speed: 7.5 + random() * 1.7,
  }));
  const dummy = new Object3D();
  const slant = new Matrix4().makeRotationZ(-0.17);
  const update = (elapsed: number): void => {
    for (let index = 0; index < drops.length; index += 1) {
      const drop = drops[index]!;
      const y = 6 - ((drop.phase + elapsed * drop.speed) % 6.5);
      dummy.position.set(drop.x + elapsed * 0.18, y, drop.z + elapsed * 0.09);
      dummy.quaternion.copy(camera.quaternion);
      dummy.updateMatrix();
      dummy.matrix.multiply(slant);
      mesh.setMatrixAt(index, dummy.matrix);
    }
    mesh.instanceMatrix.needsUpdate = true;
  };
  update(0);
  scene.add(mesh);
  return { mesh, update };
}

function makeFogOverlay(): {
  scene: Scene;
  camera: OrthographicCamera;
  material: ShaderMaterial;
} {
  const scene = new Scene();
  const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const material = new ShaderMaterial({
    uniforms: {
      elapsed: { value: 0 },
      fogColour: { value: new Vector4(0.33, 0.39, 0.52, 0.16) },
    },
    vertexShader: fogVertexShader,
    fragmentShader: fogFragmentShader,
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  scene.add(new Mesh(new PlaneGeometry(2, 2), material));
  return { scene, camera, material };
}

function lookAtCameraAroundY(mesh: Mesh, camera: OrthographicCamera): void {
  mesh.lookAt(camera.position.x, mesh.position.y, camera.position.z);
}

export async function startFeelScene(
  stage: HTMLElement,
  packet: VerifiedAssetPacket,
  presets: readonly Preset[],
): Promise<FeelSceneHandle> {
  const canvas = document.createElement("canvas");
  canvas.setAttribute("aria-hidden", "true");
  const context = canvas.getContext("webgl2", {
    alpha: false,
    antialias: true,
    depth: true,
    stencil: false,
  });
  if (context === null) throw new Error("WebGL2 is unavailable in this browser");
  stage.prepend(canvas);

  const renderer = new WebGLRenderer({ canvas, context, antialias: true });
  renderer.outputColorSpace = SRGBColorSpace;
  renderer.toneMapping = NoToneMapping;
  const palette = paletteFor(presets);
  renderer.shadowMap.enabled = true;
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.setSize(window.innerWidth, window.innerHeight, false);

  const manifest = packet.manifest;
  const textures = await decodeTextures(packet);
  const anisotropy = renderer.capabilities.getMaxAnisotropy();
  const scene = new Scene();
  scene.background = palette.background;
  const camera = createFeelCamera(window.innerWidth, window.innerHeight, manifest.layout.grid_extents);
  const lanternPosition = new Vector3().fromArray(manifest.layout.light_sources.lantern_glass);

  addGround(scene, manifest, textures, presets, palette, anisotropy);
  addWalls(scene, manifest, textures, anisotropy);
  const { lantern, lanternBase } = addLights(scene, manifest, presets, palette);
  const { billboards, swayMaterials, caretaker } = addProps(
    scene,
    manifest,
    textures,
    presets,
    palette,
    anisotropy,
    lanternPosition,
  );
  billboards.forEach((billboard) => lookAtCameraAroundY(billboard, camera));
  const rain = presets.includes("rain") ? addRain(scene, camera) : null;
  const fog = presets.includes("fog") ? makeFogOverlay() : null;
  const clock = new Clock();
  const walkPresenter = createWalkPresenter({
    stage,
    canvas,
    scene,
    camera,
    layout: manifest.layout,
    caretaker,
    startedAt: performance.now() / 1000,
  });
  let animationFrame = 0;
  let stopped = false;

  const draw = (): void => {
    if (stopped) return;
    const elapsed = clock.getElapsedTime();
    const noise = Math.sin(elapsed * 5.7 + 1.731) * 0.055 + Math.sin(elapsed * 11.3 + 2.943) * 0.025;
    lantern.intensity = lanternBase * (1 + noise);
    swayMaterials.forEach((material) => {
      material.uniforms.elapsed!.value = elapsed;
      material.uniforms.lanternStrength!.value = palette.practicalShaderStrength * (1 + noise);
    });
    rain?.update(elapsed);
    walkPresenter.update(performance.now() / 1000);
    renderer.setRenderTarget(null);
    renderer.clear();
    renderer.render(scene, camera);
    if (fog !== null) {
      fog.material.uniforms.elapsed!.value = elapsed;
      renderer.autoClear = false;
      renderer.render(fog.scene, fog.camera);
      renderer.autoClear = true;
    }
    animationFrame = requestAnimationFrame(draw);
  };

  const resize = (): void => {
    renderer.setSize(window.innerWidth, window.innerHeight, false);
    resizeFeelCamera(camera, window.innerWidth, window.innerHeight);
  };
  window.addEventListener("resize", resize);
  draw();
  return {
    renderer,
    camera,
    stop: () => {
      stopped = true;
      cancelAnimationFrame(animationFrame);
      window.removeEventListener("resize", resize);
      walkPresenter.stop();
      renderer.dispose();
    },
  };
}
