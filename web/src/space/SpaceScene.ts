import { addStructures } from "./structures";
import type { FigureFacing } from "../walk/facing";
import {
  AdditiveBlending,
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  ClampToEdgeWrapping,
  Color,
  DataTexture,
  DirectionalLight,
  DoubleSide,
  DynamicDrawUsage,
  Group,
  InstancedBufferAttribute,
  InstancedMesh,
  LinearFilter,
  Matrix4,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  Object3D,
  PlaneGeometry,
  PointLight,
  RedFormat,
  ShaderMaterial,
  ShadowMaterial,
  UnsignedByteType,
  Vector2,
  Vector3,
  type Material,
  type OrthographicCamera,
} from "three";
import { CAMERA_OFFSET, projectedHeightCoverTiles } from "../camera";
import { createFigureInstance, type DecodedFigure, type FigureInstance } from "./figureRig";
import type { FeelSpace, PropPlacement, WallRun } from "../feelTypes";
import { GRASS_CLUMP_HEIGHT, scatterGrassClumps } from "../grassClumps";
import { buildGroundGeometry } from "../groundGeometry";
import {
  buildHearthGeometry,
  HEARTH_PROFILE,
  hearthFireAnchor,
  hearthLightPosition,
  type HearthMaterial,
} from "../hearthGeometry";
import { windPresetSettings, type Preset, type WindPresetSettings } from "../presets";
import {
  buildRoofGeometry,
  mergeGeometryData,
  ROOF_SHINGLE_SLOPE_UV_SCALE,
  ROOF_SHINGLE_SLOPE_VALUE_MULTIPLIER,
  type RoofMaterial,
} from "../roofGeometry";
import {
  groundFragmentShader,
  groundVertexShader,
  hearthEmberFragmentShader,
  hearthEmberVertexShader,
  hearthFireFragmentShader,
  hearthFireVertexShader,
  hearthFlicker,
  windFragmentShader,
  windVertexShader,
} from "../shaders";
import { buildWindWeight } from "../windWeight";
import { buildWallProfile, WALL_PROFILE, type WallMaterial } from "../wallGeometry";
import type { Cell } from "../walk/layoutPassability";
import { occludingRuns } from "../walk/wallOcclusion";
import { nearWallRunIndices } from "./interiorWalls";
import { applyCardLighting } from "./cardLighting";
import { paletteFor, type ScenePalette } from "./palette";
import { propCardTransform } from "./propCards";
import {
  configureTexture,
  geometryFromData,
  requiredTexture,
  type DecodedTexture,
} from "./textures";

interface WallRunPresentation {
  run: WallRun;
  materials: Record<WallMaterial, MeshStandardMaterial>;
  fadeableMeshes: Mesh[];
  fadeAmount: number;
  fadeStartedAt: number;
  fadeStartedFrom: number;
  fadeTarget: number;
}

interface RainSystem {
  update(elapsed: number): void;
}

interface HearthPresentation {
  fireMaterial: ShaderMaterial;
  emberMaterial: ShaderMaterial;
  light: PointLight;
  lightBase: number;
}


export interface SpaceSceneOptions {
  name: string;
  space: FeelSpace;
  textures: Map<string, DecodedTexture>;
  windWeightTextures: Map<string, DataTexture>;
  presets: readonly Preset[];
  anisotropy: number;
  camera: OrthographicCamera;
  caretakerCell: Cell;
  caretakerFacing: FigureFacing;
  figures: Map<string, DecodedFigure>;
  caretakerFigure: string;
  structures: ReadonlyMap<string, Group>;
}

const WARM_LIGHT = new Color("#ffb457");
const RAIN_COUNT = 1080;
/** A frame gap beyond this is a pause, not a slow frame. */
const PAUSE_GAP_SECONDS = 2;
const WALL_FADE_DURATION_SECONDS = 0.35;
const WALL_FADED_PLASTER_OPACITY = 0.34;
const WALL_FADED_TIMBER_OPACITY = 0.48;
const WALL_FADED_RENDER_ORDER = 10;
const WALL_COVER_TILES = projectedHeightCoverTiles(WALL_PROFILE.capTop);
export const HEARTH_LIGHT_INTENSITY_MULTIPLIER = 8;
export const HEARTH_LIGHT_DISTANCE = 7.5;

interface SharedWindUniforms {
  elapsed: { value: number };
  windDirection: { value: Vector2 };
  windStrength: { value: number };
  gustPeriod: { value: number };
}

function cachedWindWeightTexture(
  cache: Map<string, DataTexture>,
  kind: string,
  source: DecodedTexture,
): DataTexture {
  const cached = cache.get(kind);
  if (cached !== undefined) return cached;
  if (source.pixels === null) {
    throw new Error(`wind texture ${kind} was decoded without readable pixels`);
  }
  const weights = buildWindWeight(
    { width: source.width, height: source.height, data: source.pixels },
    kind,
  );
  const texture = new DataTexture(
    weights,
    source.width,
    source.height,
    RedFormat,
    UnsignedByteType,
  );
  texture.name = `wind-weight-${kind}`;
  texture.flipY = true;
  texture.wrapS = ClampToEdgeWrapping;
  texture.wrapT = ClampToEdgeWrapping;
  texture.magFilter = LinearFilter;
  texture.minFilter = LinearFilter;
  texture.generateMipmaps = false;
  texture.unpackAlignment = 1;
  texture.needsUpdate = true;
  cache.set(kind, texture);
  return texture;
}

function wallMaterials(
  textures: Map<string, DecodedTexture>,
  anisotropy: number,
  runIndex: number,
): Record<WallMaterial, MeshStandardMaterial> {
  const build = (name: WallMaterial, cutout = false): MeshStandardMaterial => {
    const map = requiredTexture(textures, `walls/${name}`).texture;
    configureTexture(map, anisotropy);
    return new MeshStandardMaterial({
      name: `wall-run-${runIndex}-${name}`,
      map,
      roughness: 0.86,
      metalness: 0,
      alphaTest: cutout ? 0.12 : 0,
      side: cutout ? DoubleSide : undefined,
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

function easeOutCubic(progress: number): number {
  return 1 - (1 - progress) ** 3;
}

function fadeAmountAt(run: WallRunPresentation, now: number): number {
  if (run.fadeAmount === run.fadeTarget) return run.fadeTarget;
  const progress = Math.min(
    1,
    Math.max(0, (now - run.fadeStartedAt) / WALL_FADE_DURATION_SECONDS),
  );
  return run.fadeStartedFrom +
    (run.fadeTarget - run.fadeStartedFrom) * easeOutCubic(progress);
}

function applyWallFade(run: WallRunPresentation, amount: number): void {
  const faded = amount > 0;
  for (const [name, material] of Object.entries(run.materials) as [
    WallMaterial,
    MeshStandardMaterial,
  ][]) {
    if (name === "plinth") continue;
    const fadedOpacity = name === "plaster"
      ? WALL_FADED_PLASTER_OPACITY
      : WALL_FADED_TIMBER_OPACITY;
    material.opacity = 1 + (fadedOpacity - 1) * amount;
    if (material.transparent !== faded) {
      material.transparent = faded;
      material.needsUpdate = true;
    }
    material.depthWrite = !faded;
  }
  for (const mesh of run.fadeableMeshes) {
    mesh.renderOrder = faded ? WALL_FADED_RENDER_ORDER : 0;
  }
}

function addContactShadow(group: Group, x: number, z: number, height: number): Mesh {
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
  shadow.position.set(x, 0.004, z);
  group.add(shadow);
  return shadow;
}

function seededRandom(seed = 0x544d455f): () => number {
  let state = seed >>> 0;
  return () => {
    state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
    return state / 0x100000000;
  };
}

function addRain(
  group: Group,
  camera: OrthographicCamera,
  extents: FeelSpace["grid_extents"],
): RainSystem {
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
    x: -3 + random() * (extents.i + 6),
    z: -3 + random() * (extents.j + 6),
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
  group.add(mesh);
  return { update };
}

function disposeMaterial(material: Material): void {
  material.dispose();
}

export class SpaceScene {
  readonly group = new Group();
  readonly caretaker: FigureInstance;
  readonly background: Color;
  readonly weatherEnabled: boolean;
  readonly grassInstanceCount: number;

  private readonly palette: ScenePalette;
  private readonly windMaterials: ShaderMaterial[] = [];
  private readonly windSettings: WindPresetSettings;
  private readonly windUniforms: SharedWindUniforms;
  private readonly wallRuns: WallRunPresentation[] = [];
  private readonly hearths: HearthPresentation[] = [];
  private readonly keyLight: DirectionalLight;
  private readonly keyTarget: Object3D;
  /** World direction toward the key light; constant while it tracks focus. */
  private readonly keyDirection: Vector3;
  private readonly lantern: PointLight | null;
  private readonly lanternBase: number;
  private readonly rain: RainSystem | null;
  private readonly interior: boolean;

  constructor(private readonly options: SpaceSceneOptions) {
    const { name, space, textures, presets, anisotropy, caretakerCell } = options;
    this.group.name = `Space_${name}`;
    this.weatherEnabled = space.weather;
    this.interior = space.roofs.length === 0;
    this.palette = paletteFor(presets, space.weather);
    this.windSettings = windPresetSettings(presets, space.weather);
    this.windUniforms = {
      elapsed: { value: 0 },
      windDirection: { value: new Vector2(...this.windSettings.direction).normalize() },
      windStrength: { value: this.windSettings.strength },
      gustPeriod: { value: this.windSettings.gustPeriod },
    };
    this.background = this.palette.background;
    this.addGround();
    this.addWalls();
    this.addRoofs();
    addStructures(this.group, name, space.structures, options.structures);
    this.addFixtures();
    const lights = this.addLights(caretakerCell);
    this.keyLight = lights.key;
    this.keyTarget = lights.target;
    this.keyDirection = lights.direction;
    this.lantern = lights.lantern;
    this.lanternBase = lights.lanternBase;
    this.addProps();
    this.caretaker = this.addCaretaker();
    this.grassInstanceCount = this.addGrass();
    this.rain = space.weather && presets.includes("rain")
      ? addRain(this.group, options.camera, space.grid_extents)
      : null;
  }

  private addGround(): void {
    const { space, textures, presets, anisotropy } = this.options;
    const rainy = space.weather && presets.includes("rain");
    const ambient = this.palette.ambient.clone().multiplyScalar(this.palette.ambientIntensity);
    const key = this.palette.key.clone().multiplyScalar(this.palette.keyIntensity * 0.44);
    const cellsByMaterial = new Map<string, typeof space.cells>();
    for (const cell of space.cells) {
      const cells = cellsByMaterial.get(cell.material) ?? [];
      cells.push(cell);
      cellsByMaterial.set(cell.material, cells);
    }
    for (const [materialName, cells] of cellsByMaterial) {
      const swatch = requiredTexture(textures, `terrain/${materialName}`).texture;
      configureTexture(swatch, anisotropy);
      const material = new ShaderMaterial({
        name: `ground-${materialName}`,
        uniforms: {
          swatch: { value: swatch },
          swatchPeriod: { value: 3 },
          jointWidth: { value: 0.028 },
          wetness: { value: rainy ? 1 : 0 },
          timeTint: {
            value: space.weather && presets.includes("dusk")
              ? new Color(0.94, 0.94, 0.94)
              : materialName === "grass"
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
      const data = buildGroundGeometry(cells);
      const geometry = new BufferGeometry();
      geometry.setAttribute("position", new BufferAttribute(new Float32Array(data.positions), 3));
      geometry.setAttribute("uv", new BufferAttribute(new Float32Array(data.uvs), 2));
      geometry.setAttribute("cellOrigin", new BufferAttribute(new Float32Array(data.cellOrigins), 2));
      geometry.setIndex(data.indices);
      geometry.computeVertexNormals();
      geometry.computeBoundingSphere();
      const mesh = new Mesh(geometry, material);
      mesh.name = `Ground_${materialName}`;
      this.group.add(mesh);
    }

    const extents = space.grid_extents;
    const shadowPlane = new Mesh(
      new PlaneGeometry(extents.i, extents.j),
      new ShadowMaterial({ color: 0x02050b, opacity: 0.34 }),
    );
    shadowPlane.name = "GroundShadowReceiver";
    shadowPlane.rotation.x = -Math.PI / 2;
    shadowPlane.position.set((extents.i - 1) / 2, -0.003, (extents.j - 1) / 2);
    shadowPlane.receiveShadow = true;
    this.group.add(shadowPlane);
  }

  private addWalls(): void {
    const { space, textures, anisotropy } = this.options;
    const nearRuns = nearWallRunIndices(space);
    space.wall_runs.forEach((run, runIndex) => {
      this.wallRuns.push({
        run,
        materials: wallMaterials(textures, anisotropy, runIndex),
        fadeableMeshes: [],
        fadeAmount: 0,
        fadeStartedAt: 0,
        fadeStartedFrom: 0,
        fadeTarget: 0,
      });
    });
    for (const part of buildWallProfile(space.wall_runs)) {
      if (nearRuns.has(part.runIndex) && part.material !== "plinth" && part.material !== "sill") {
        continue;
      }
      const run = this.wallRuns[part.runIndex];
      if (run === undefined) throw new Error(`wall part ${part.label} names absent run ${part.runIndex}`);
      const mesh = new Mesh(geometryFromData(part.geometry), run.materials[part.material]);
      mesh.name = `WallRun_${part.runIndex}_${part.label}`;
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      if (part.material !== "plinth") run.fadeableMeshes.push(mesh);
      this.group.add(mesh);
    }
  }

  private addRoofs(): void {
    const { space, textures, anisotropy } = this.options;
    const batches = new Map<
      string,
      { material: RoofMaterial; textureKey: string; geometries: ReturnType<typeof buildRoofGeometry>[number]["geometry"][] }
    >();
    for (const roof of space.roofs) {
      for (const part of buildRoofGeometry(roof)) {
        const textureKey = part.material === "plaster" || part.material === "post"
          ? `walls/${part.material}`
          : `roofs/${roof.material}_${part.material.replace("shingle_", "")}`;
        const key = `${part.material}:${textureKey}`;
        const batch = batches.get(key) ?? {
          material: part.material,
          textureKey,
          geometries: [],
        };
        batch.geometries.push(part.geometry);
        batches.set(key, batch);
      }
    }
    for (const [key, batch] of batches) {
      const map = requiredTexture(textures, batch.textureKey).texture;
      configureTexture(map, anisotropy);
      if (batch.material === "shingle_slope") {
        map.repeat.setScalar(1 / ROOF_SHINGLE_SLOPE_UV_SCALE);
        map.needsUpdate = true;
      }
      const material = new MeshStandardMaterial({
        name: `roof-${key}`,
        map,
        color: batch.material === "shingle_slope"
          ? new Color(
              ROOF_SHINGLE_SLOPE_VALUE_MULTIPLIER,
              ROOF_SHINGLE_SLOPE_VALUE_MULTIPLIER,
              ROOF_SHINGLE_SLOPE_VALUE_MULTIPLIER,
            )
          : undefined,
        roughness: batch.material.startsWith("shingle_") ? 0.92 : 0.86,
        side: DoubleSide,
      });
      const mesh = new Mesh(
        geometryFromData(mergeGeometryData(batch.geometries)),
        material,
      );
      mesh.name = `RoofBatch_${batch.material}`;
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
    }
  }

  private addFixtures(): void {
    const { space, textures, anisotropy } = this.options;
    if (space.fixtures.length === 0) return;

    const partsByMaterial = new Map<
      HearthMaterial,
      ReturnType<typeof buildHearthGeometry>[number]["geometry"][]
    >();
    for (const part of buildHearthGeometry(space.fixtures)) {
      const geometries = partsByMaterial.get(part.material) ?? [];
      geometries.push(part.geometry);
      partsByMaterial.set(part.material, geometries);
    }
    const fieldstone = requiredTexture(textures, "walls/fieldstone").texture;
    const timber = requiredTexture(textures, "walls/post").texture;
    configureTexture(fieldstone, anisotropy);
    configureTexture(timber, anisotropy);
    for (const [materialName, geometries] of partsByMaterial) {
      const material = new MeshStandardMaterial({
        name: `hearth-${materialName}`,
        map: materialName === "post" ? timber : fieldstone,
        color: materialName === "fieldstone_dark"
          ? new Color(0.35, 0.35, 0.35)
          : undefined,
        roughness: materialName === "post" ? 0.86 : 0.94,
        metalness: 0,
      });
      const mesh = new Mesh(geometryFromData(mergeGeometryData(geometries)), material);
      mesh.name = `HearthBatch_${materialName}`;
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
    }

    const source = requiredTexture(textures, "props/fire");
    configureTexture(source.texture, anisotropy);
    const cardRotation = Math.atan2(CAMERA_OFFSET.x, CAMERA_OFFSET.z);
    space.fixtures.forEach((fixture, fixtureIndex) => {
      const anchor = hearthFireAnchor(fixture);
      const fireGeometry = new PlaneGeometry(
        HEARTH_PROFILE.fireHeight * (source.width / source.height),
        HEARTH_PROFILE.fireHeight,
      );
      const fireMaterial = new ShaderMaterial({
        name: `hearth-fire-${fixtureIndex}`,
        uniforms: {
          albedoTexture: { value: source.texture },
          elapsed: { value: 0 },
          flicker: { value: 1 },
        },
        vertexShader: hearthFireVertexShader,
        fragmentShader: hearthFireFragmentShader,
        transparent: true,
        alphaTest: 0.12,
        side: DoubleSide,
      });
      const fire = new Mesh(fireGeometry, fireMaterial);
      fire.name = `HearthFire_${fixtureIndex}`;
      fire.position.fromArray(anchor.position);
      fire.rotation.y = cardRotation;
      this.group.add(fire);

      const emberGeometry = new PlaneGeometry(1, 1);
      const emberCount = 24;
      const phases = new Float32Array(emberCount);
      const drifts = new Float32Array(emberCount);
      const rises = new Float32Array(emberCount);
      emberGeometry.setAttribute("emberPhase", new InstancedBufferAttribute(phases, 1));
      emberGeometry.setAttribute("emberDrift", new InstancedBufferAttribute(drifts, 1));
      emberGeometry.setAttribute("emberRise", new InstancedBufferAttribute(rises, 1));
      const emberMaterial = new ShaderMaterial({
        name: `hearth-embers-${fixtureIndex}`,
        uniforms: {
          elapsed: { value: 0 },
          fixtureLateral: { value: new Vector3().fromArray(anchor.lateral) },
        },
        vertexShader: hearthEmberVertexShader,
        fragmentShader: hearthEmberFragmentShader,
        transparent: true,
        depthWrite: false,
        blending: AdditiveBlending,
        side: DoubleSide,
      });
      const embers = new InstancedMesh(emberGeometry, emberMaterial, emberCount);
      embers.name = `HearthEmbers_${fixtureIndex}`;
      embers.instanceMatrix.setUsage(DynamicDrawUsage);
      embers.frustumCulled = false;
      const random = seededRandom(0x48454152 + fixtureIndex);
      const dummy = new Object3D();
      for (let index = 0; index < emberCount; index += 1) {
        phases[index] = index / emberCount;
        drifts[index] = (random() * 2 - 1) * 0.035;
        rises[index] = 0.4 + random() * 0.18;
        const lateral = (random() * 2 - 1) * (HEARTH_PROFILE.fireboxWidth * 0.36);
        dummy.position.set(
          anchor.position[0] + anchor.lateral[0] * lateral,
          HEARTH_PROFILE.fireboxSill + 0.025,
          anchor.position[2] + anchor.lateral[2] * lateral,
        );
        dummy.rotation.y = cardRotation;
        dummy.scale.set(0.012 + random() * 0.012, 0.025 + random() * 0.025, 1);
        dummy.updateMatrix();
        embers.setMatrixAt(index, dummy.matrix);
      }
      embers.instanceMatrix.needsUpdate = true;
      this.group.add(embers);

      const lightBase = this.palette.candleIntensity * HEARTH_LIGHT_INTENSITY_MULTIPLIER;
      const light = new PointLight(WARM_LIGHT, lightBase, HEARTH_LIGHT_DISTANCE, 2);
      light.name = `HearthFireLight_${fixtureIndex}`;
      light.position.fromArray(hearthLightPosition(fixture));
      light.castShadow = true;
      light.shadow.mapSize.set(512, 512);
      light.shadow.camera.near = 0.05;
      light.shadow.camera.far = HEARTH_LIGHT_DISTANCE;
      light.shadow.bias = -0.0005;
      light.shadow.normalBias = 0.015;
      this.group.add(light);
      this.hearths.push({ fireMaterial, emberMaterial, light, lightBase });
    });
  }

  private addLights(focusCell: Cell): {
    key: DirectionalLight;
    target: Object3D;
    direction: Vector3;
    lantern: PointLight | null;
    lanternBase: number;
  } {
    const { space, presets } = this.options;
    this.group.add(new AmbientLight(this.palette.ambient, this.palette.ambientIntensity));
    const key = new DirectionalLight(this.palette.key, this.palette.keyIntensity);
    key.name = space.weather && presets.includes("dusk") ? "WarmHorizonKey" : "CoolMoonlight";
    const keyOffset = space.weather && presets.includes("dusk")
      ? new Vector3(-10.5, 6, 6.5)
      : new Vector3(3.5, 12, -10.5);
    key.target.position.set(focusCell.i, 0, focusCell.j);
    key.position.copy(key.target.position).add(keyOffset);
    key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    key.shadow.camera.left = -10;
    key.shadow.camera.right = 10;
    key.shadow.camera.top = 10;
    key.shadow.camera.bottom = -10;
    key.shadow.camera.near = 0.1;
    key.shadow.camera.far = 40;
    key.shadow.bias = -0.00025;
    this.group.add(key, key.target);

    let lantern: PointLight | null = null;
    if (space.light_sources.lantern_glass !== null) {
      lantern = new PointLight(WARM_LIGHT, this.palette.lanternIntensity, 6, 2);
      lantern.name = "LanternGlow";
      lantern.position.fromArray(space.light_sources.lantern_glass);
      lantern.castShadow = true;
      lantern.shadow.mapSize.set(512, 512);
      this.group.add(lantern);
    }
    space.light_sources.candles.forEach((position, index) => {
      const candle = new PointLight(WARM_LIGHT, this.palette.candleIntensity, 2.2, 2);
      candle.name = `Candle_${index}`;
      candle.position.fromArray(position);
      this.group.add(candle);
    });
    return {
      key,
      target: key.target,
      direction: keyOffset.clone().normalize(),
      lantern,
      lanternBase: this.palette.lanternIntensity,
    };
  }

  private addCaretaker(): FigureInstance {
    const { figures, caretakerFigure, caretakerCell, caretakerFacing } = this.options;
    const figure = figures.get(caretakerFigure);
    if (figure === undefined) throw new Error(`the packet's caretaker figure ${caretakerFigure} was not decoded`);
    const instance = createFigureInstance(figure, caretakerCell, caretakerFacing);
    this.group.add(instance.root);
    return instance;
  }

  private addProps(): void {
    const { space, textures, anisotropy } = this.options;
    const placements = space.props;
    for (const prop of placements) {
      const source = requiredTexture(textures, `props/${prop.kind}`);
      configureTexture(source.texture, anisotropy);
      // Size agreement with the colour sheet was proven at decode time.
      const normal = textures.get(`props/${prop.kind}/normal`) ?? null;
      if (normal !== null) configureTexture(normal.texture, anisotropy);
      const width = prop.card_height * (source.width / source.height);
      const geometry = new PlaneGeometry(width, prop.card_height);
      let material: ShaderMaterial | MeshStandardMaterial;
      if (prop.sway) {
        material = this.createWindMaterial(
          prop.kind,
          source,
          normal,
          new Vector2(prop.cell_anchor[0], prop.cell_anchor[1]),
          `wind-${prop.kind}`,
        );
      } else {
        material = new MeshStandardMaterial({
          name: `prop-${prop.kind}`,
          map: source.texture,
          normalMap: normal?.texture ?? null,
          transparent: true,
          alphaTest: 0.12,
          roughness: 0.88,
          metalness: 0,
          side: DoubleSide,
        });
        applyCardLighting(material);
      }
      const mesh = new Mesh(geometry, material);
      mesh.name = `Prop_${prop.kind}`;
      const transform = propCardTransform(prop);
      mesh.scale.x = transform.scaleX;
      mesh.position.set(transform.position.x, transform.position.y, transform.position.z);
      mesh.rotation.set(transform.rotationX, transform.rotationY, 0, "YXZ");
      mesh.castShadow = true;
      const contactShadow = addContactShadow(
        this.group,
        transform.position.x,
        transform.position.z,
        prop.card_height,
      );
      const shadowRotation = transform.contactShadowRotation;
      contactShadow.rotation.set(
        shadowRotation.x,
        shadowRotation.y,
        shadowRotation.z,
        shadowRotation.order,
      );
      this.group.add(mesh);
      void contactShadow;
    }
  }

  private createWindMaterial(
    kind: string,
    source: DecodedTexture,
    normal: DecodedTexture | null,
    worldAnchor: Vector2,
    name: string,
  ): ShaderMaterial {
    const lanternPosition = this.options.space.light_sources.lantern_glass === null
      ? new Vector3(0, 0, 0)
      : new Vector3().fromArray(this.options.space.light_sources.lantern_glass);
    const material = new ShaderMaterial({
      name,
      defines: normal === null ? {} : { CARD_NORMAL_MAP: "" },
      uniforms: {
        albedoTexture: { value: source.texture },
        normalTexture: { value: normal?.texture ?? null },
        keyDirection: { value: this.keyDirection.clone() },
        windWeightTexture: {
          value: cachedWindWeightTexture(
            this.options.windWeightTextures,
            kind,
            source,
          ),
        },
        elapsed: this.windUniforms.elapsed,
        windDirection: this.windUniforms.windDirection,
        windStrength: this.windUniforms.windStrength,
        gustPeriod: this.windUniforms.gustPeriod,
        worldAnchor: { value: worldAnchor },
        ambientColour: {
          value: this.palette.ambient.clone().multiplyScalar(this.palette.ambientIntensity),
        },
        keyColour: {
          value: this.palette.key.clone().multiplyScalar(this.palette.keyIntensity * 0.34),
        },
        lanternPosition: { value: lanternPosition },
        lanternColour: { value: WARM_LIGHT.clone() },
        lanternStrength: { value: this.palette.practicalShaderStrength },
      },
      vertexShader: windVertexShader,
      fragmentShader: windFragmentShader,
      transparent: true,
      alphaTest: 0.12,
      side: DoubleSide,
    });
    this.windMaterials.push(material);
    return material;
  }

  private addGrass(): number {
    const clumps = scatterGrassClumps(this.options.space);
    if (clumps.length === 0) return 0;
    const source = requiredTexture(this.options.textures, "props/grass_clump");
    configureTexture(source.texture, this.options.anisotropy);
    const width = GRASS_CLUMP_HEIGHT * (source.width / source.height);
    const geometry = new PlaneGeometry(width, GRASS_CLUMP_HEIGHT);
    const material = this.createWindMaterial(
      "grass_clump",
      source,
      null,
      new Vector2(),
      "wind-grass-clumps",
    );
    const mesh = new InstancedMesh(geometry, material, clumps.length);
    mesh.name = "GrassClumps";
    mesh.castShadow = false;
    mesh.receiveShadow = false;
    mesh.frustumCulled = false;
    const cardRotation = Math.atan2(CAMERA_OFFSET.x, CAMERA_OFFSET.z);
    const dummy = new Object3D();
    clumps.forEach((clump, index) => {
      dummy.position.set(clump.x, GRASS_CLUMP_HEIGHT * clump.scale * 0.5, clump.z);
      dummy.rotation.set(0, cardRotation, 0);
      dummy.scale.set((clump.mirror ? -1 : 1) * clump.scale, clump.scale, 1);
      dummy.updateMatrix();
      mesh.setMatrixAt(index, dummy.matrix);
    });
    mesh.instanceMatrix.needsUpdate = true;
    this.group.add(mesh);
    return clumps.length;
  }

  focusLighting(previous: Cell, next: Cell): void {
    const deltaI = next.i - previous.i;
    const deltaJ = next.j - previous.j;
    this.keyLight.position.x += deltaI;
    this.keyLight.position.z += deltaJ;
    this.keyTarget.position.x += deltaI;
    this.keyTarget.position.z += deltaJ;
    this.keyLight.updateMatrixWorld(true);
    this.keyTarget.updateMatrixWorld(true);
  }

  updateWallFade(playerCell: Cell, now: number): number {
    if (this.interior) return 0;
    const selected = new Set(
      occludingRuns(this.options.space.wall_runs, playerCell, WALL_COVER_TILES),
    );
    let fadedRuns = 0;
    for (const run of this.wallRuns) {
      const target = selected.has(run.run) ? 1 : 0;
      if (target === 1) fadedRuns += 1;
      if (target !== run.fadeTarget) {
        const current = fadeAmountAt(run, now);
        run.fadeAmount = current;
        run.fadeStartedFrom = current;
        run.fadeStartedAt = now;
        run.fadeTarget = target;
      }
      run.fadeAmount = fadeAmountAt(run, now);
      applyWallFade(run, run.fadeAmount);
    }
    return fadedRuns;
  }

  wallRunPlasterOpacity(runIndex: number): number | null {
    return this.wallRuns[runIndex]?.materials.plaster.opacity ?? null;
  }

  private lastElapsed = 0;

  update(elapsed: number): void {
    const delta = elapsed - this.lastElapsed;
    this.lastElapsed = elapsed;
    // The clips keep wall-clock time with the root, however slow the frame,
    // or the feet slide; only a gap long enough to be a pause — a hidden tab,
    // not a slow rasteriser — is treated as one and advanced a little.
    if (delta > 0) this.caretaker.update(delta <= PAUSE_GAP_SECONDS ? delta : 0.5);
    this.windUniforms.elapsed.value = elapsed;
    this.windUniforms.windDirection.value.set(...this.windSettings.direction).normalize();
    this.windUniforms.windStrength.value = this.windSettings.strength;
    this.windUniforms.gustPeriod.value = this.windSettings.gustPeriod;
    const noise = Math.sin(elapsed * 5.7 + 1.731) * 0.055 +
      Math.sin(elapsed * 11.3 + 2.943) * 0.025;
    if (this.lantern !== null) this.lantern.intensity = this.lanternBase * (1 + noise);
    for (const material of this.windMaterials) {
      material.uniforms.lanternStrength!.value =
        this.palette.practicalShaderStrength * (1 + noise);
    }
    const fireFlicker = hearthFlicker(elapsed);
    for (const hearth of this.hearths) {
      hearth.fireMaterial.uniforms.elapsed!.value = elapsed;
      hearth.fireMaterial.uniforms.flicker!.value = fireFlicker;
      hearth.emberMaterial.uniforms.elapsed!.value = elapsed;
      hearth.light.intensity = hearth.lightBase * fireFlicker;
    }
    this.rain?.update(elapsed);
  }

  dispose(): void {
    this.caretaker.dispose();
    const geometries = new Set<BufferGeometry>();
    const materials = new Set<Material>();
    this.group.traverse((object) => {
      if (!(object instanceof Mesh) || object.userData.sharedStructure) return;
      geometries.add(object.geometry);
      if (Array.isArray(object.material)) object.material.forEach((material) => materials.add(material));
      else materials.add(object.material);
    });
    for (const run of this.wallRuns) {
      Object.values(run.materials).forEach((material) => materials.add(material));
    }
    geometries.forEach((geometry) => geometry.dispose());
    materials.forEach(disposeMaterial);
    this.group.removeFromParent();
    this.group.clear();
  }
}
