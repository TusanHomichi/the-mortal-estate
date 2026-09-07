import { createSurfaceOcclusion, type OccludingSurface } from "./surfaceOcclusion";
import { textureHitTest } from "./surfaceAlpha";
import { createTacticalGrid } from "./tacticalGrid";
import { createStructureAtmosphere } from "./structureAtmosphere";
import { resolveInteriorLighting } from "./structureLighting";
import { addGrassCover, type GrassCover } from "./grassCover";
import { addStructures } from "./structures";
import type { FigureFacing } from "../walk/facing";
import {
  AdditiveBlending,
  AmbientLight,
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
  Light,
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
  UnsignedByteType,
  Vector2,
  Vector3,
  type Material,
  type OrthographicCamera,
} from "three";
import { CAMERA_OFFSET } from "../camera";
import { createFigureInstance, type DecodedFigure, type FigureInstance } from "./figureRig";
import type { FeelSpace } from "../feelTypes";
import { GRASS_CLUMP_HEIGHT, scatterGrassClumps } from "../grassClumps";
import { createTerrainSurface, type TerrainSurface } from "../terrainSurface";
import { addGround } from "./ground";
import {
  buildHearthGeometry,
  HEARTH_PROFILE,
  hearthFireAnchor,
  hearthLightPosition,
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
  hearthEmberFragmentShader,
  hearthEmberVertexShader,
  hearthFireFragmentShader,
  hearthFireVertexShader,
  hearthFlicker,
  windFragmentShader,
  windVertexShader,
} from "../shaders";
import { buildWindWeight } from "../windWeight";
import { buildWallProfile, type WallMaterial } from "../wallGeometry";
import type { Cell } from "../walk/layoutPassability";
import { nearWallRunIndices } from "./interiorWalls";
import { applyCardLighting } from "./cardLighting";
import { keyLightOffset, paletteFor, type ScenePalette } from "./palette";
import { propCardTransform } from "./propCards";
import {
  configureTexture,
  geometryFromData,
  requiredTexture,
  type DecodedTexture,
} from "./textures";

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
  private readonly wallRuns: Record<WallMaterial, MeshStandardMaterial>[] = [];
  private readonly occludingSurfaces: OccludingSurface[] = [];
  private readonly occlusion: ReturnType<typeof createSurfaceOcclusion>;
  private readonly hearths: HearthPresentation[] = [];
  private readonly keyLight: DirectionalLight | null;
  private readonly keyTarget: Object3D;
  /** World direction toward the key light; constant while it tracks focus. */
  private readonly keyDirection: Vector3;
  private readonly lantern: PointLight | null;
  private readonly lanternBase: number;
  private readonly rain: RainSystem | null;
  private readonly surface: TerrainSurface;
  private readonly structureAtmosphere: ReturnType<typeof createStructureAtmosphere>;
  private readonly tacticalGrid: ReturnType<typeof createTacticalGrid>;
  private grassCover: GrassCover | null = null;

  constructor(private readonly options: SpaceSceneOptions) {
    const { name, space, presets, caretakerCell } = options;
    this.group.name = `Space_${name}`;
    this.weatherEnabled = space.weather;
    const lighting = resolveInteriorLighting(space.structures.map((_, index) =>
      options.structures.get(`structures/${name}/${index}`)).filter((root): root is Group => root !== undefined));
    this.palette = paletteFor(presets, space.weather, lighting);
    this.windSettings = windPresetSettings(presets, space.weather);
    this.windUniforms = {
      elapsed: { value: 0 },
      windDirection: { value: new Vector2(...this.windSettings.direction).normalize() },
      windStrength: { value: this.windSettings.strength },
      gustPeriod: { value: this.windSettings.gustPeriod },
    };
    this.background = this.palette.background;
    this.surface = createTerrainSurface(space);
    addGround(this.group, options, this.palette, this.windUniforms.elapsed, this.surface);
    this.tacticalGrid = createTacticalGrid(space, this.surface);
    this.group.add(this.tacticalGrid.mesh);
    this.addWalls();
    this.addRoofs();
    addStructures(this.group, name, space.structures, options.structures, this.surface.heightAt);
    this.structureAtmosphere = createStructureAtmosphere(this.group, presets, options.camera, lighting);
    this.group.traverse(object => {
      if (object instanceof Mesh && object.userData.sharedStructure) this.occludingSurfaces.push({ mesh: object });
    });
    this.addFixtures();
    const lights = this.addLights(caretakerCell);
    this.keyLight = lights.key;
    this.keyTarget = lights.target;
    this.keyDirection = lights.direction;
    this.lantern = lights.lantern;
    this.lanternBase = lights.lanternBase;
    this.addProps();
    this.caretaker = this.addCaretaker();
    this.occlusion = createSurfaceOcclusion(this.occludingSurfaces, this.caretaker.root, options.camera);
    this.grassInstanceCount = this.addGrass();
    this.rain = space.weather && presets.includes("rain")
      ? addRain(this.group, options.camera, space.grid_extents)
      : null;
  }

  private addWalls(): void {
    const { space, textures, anisotropy } = this.options;
    const nearRuns = nearWallRunIndices(space);
    space.wall_runs.forEach((_run, runIndex) => {
      this.wallRuns.push(wallMaterials(textures, anisotropy, runIndex));
    });
    for (const part of buildWallProfile(space.wall_runs)) {
      if (nearRuns.has(part.runIndex) && part.material !== "plinth" && part.material !== "sill") {
        continue;
      }
      const run = this.wallRuns[part.runIndex];
      if (run === undefined) throw new Error(`wall part ${part.label} names absent run ${part.runIndex}`);
      const mesh = new Mesh(geometryFromData(part.geometry), run[part.material]);
      mesh.name = `WallRun_${part.runIndex}_${part.label}`;
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.occludingSurfaces.push({ mesh, acceptsHit: part.material === "door"
        ? textureHitTest(requiredTexture(textures, "walls/door")) : undefined });
      this.group.add(mesh);
    }
  }

  private addRoofs(): void {
    const { space, textures, anisotropy } = this.options;
    const batches = new Map<
      string,
      { material: RoofMaterial; textureKey: string; geometries: ReturnType<typeof buildRoofGeometry>[number]["geometry"][] }
    >();
    for (const [roofIndex, roof] of space.roofs.entries()) {
      for (const part of buildRoofGeometry(roof)) {
        const textureKey = part.material === "plaster" || part.material === "post"
          ? `walls/${part.material}`
          : `roofs/${roof.material}_${part.material.replace("shingle_", "")}`;
        const key = `${roofIndex}:${part.material}:${textureKey}`;
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
      mesh.name = `RoofBatch_${key}`;
      this.occludingSurfaces.push({ mesh });
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      this.group.add(mesh);
    }
  }

  private addFixtures(): void {
    const { space, textures, anisotropy } = this.options;
    if (space.fixtures.length === 0) return;

    const fieldstone = requiredTexture(textures, "walls/fieldstone").texture;
    const timber = requiredTexture(textures, "walls/post").texture;
    configureTexture(fieldstone, anisotropy);
    configureTexture(timber, anisotropy);
    for (const part of buildHearthGeometry(space.fixtures)) {
      const materialName = part.material;
      const material = new MeshStandardMaterial({
        name: `hearth-${materialName}`,
        map: materialName === "post" ? timber : fieldstone,
        color: materialName === "fieldstone_dark"
          ? new Color(0.35, 0.35, 0.35)
          : undefined,
        roughness: materialName === "post" ? 0.86 : 0.94,
        metalness: 0,
      });
      const mesh = new Mesh(geometryFromData(part.geometry), material);
      mesh.name = `Hearth_${part.fixtureIndex}_${part.label}`;
      this.occludingSurfaces.push({ mesh });
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
    key: DirectionalLight | null;
    target: Object3D;
    direction: Vector3;
    lantern: PointLight | null;
    lanternBase: number;
  } {
    const { space, presets } = this.options;
    this.group.add(new AmbientLight(this.palette.ambient, this.palette.ambientIntensity));
    const key = this.palette.keyIntensity > 0 ? new DirectionalLight(this.palette.key, this.palette.keyIntensity) : null;
    const target = new Object3D();
    const keyOffset = keyLightOffset(presets, space.weather);
    if (key) {
      key.target = target;
      key.name = space.weather && presets.includes("dusk") ? "WarmHorizonKey" : "CoolMoonlight";
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
      key.shadow.normalBias = 0.025;
      this.group.add(key, key.target);
    }

    const lanternBase = this.palette.lanternIntensity * (this.surface.coastal ? .35 : 1);
    let lantern: PointLight | null = null;
    if (space.light_sources.lantern_glass !== null) {
      lantern = new PointLight(WARM_LIGHT, lanternBase, 6, 2);
      lantern.name = "LanternGlow";
      lantern.position.fromArray(space.light_sources.lantern_glass);
      lantern.castShadow = true;
      lantern.shadow.mapSize.set(512, 512);
      lantern.shadow.normalBias = 0.025;
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
      target,
      direction: keyOffset.clone().normalize(),
      lantern,
      lanternBase,
    };
  }

  private addCaretaker(): FigureInstance {
    const { figures, caretakerFigure, caretakerCell, caretakerFacing } = this.options;
    const figure = figures.get(caretakerFigure);
    if (figure === undefined) throw new Error(`the packet's caretaker figure ${caretakerFigure} was not decoded`);
    const instance = createFigureInstance(figure, caretakerCell, caretakerFacing, this.surface.heightAt);
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
      this.occludingSurfaces.push({ mesh, acceptsHit: textureHitTest(source) });
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
    if (this.surface.coastal) {
      this.grassCover = addGrassCover(this.group, this.options.space, this.surface, this.windUniforms);
      return this.grassCover.count;
    }
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

  focusGrid(cell: Cell | null): void {
    this.tacticalGrid.focus(cell);
  }

  focusLighting(previous: Cell, next: Cell): void {
    if (!this.keyLight) return;
    const deltaI = next.i - previous.i;
    const deltaJ = next.j - previous.j;
    this.keyLight.position.x += deltaI;
    this.keyLight.position.z += deltaJ;
    this.keyTarget.position.x += deltaI;
    this.keyTarget.position.z += deltaJ;
    this.keyLight.updateMatrixWorld(true);
    this.keyTarget.updateMatrixWorld(true);
  }

  updateOcclusion(now: number): number {
    return this.occlusion.update(now);
  }

  surfaceFades(): ReturnType<typeof this.occlusion.snapshot> {
    return this.occlusion.snapshot();
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
    this.structureAtmosphere.update(elapsed);
    this.rain?.update(elapsed);
    this.grassCover?.update(elapsed, this.caretaker.root.position);
  }

  dispose(): void {
    this.occlusion.dispose();
    this.structureAtmosphere.dispose();
    this.caretaker.dispose();
    this.grassCover?.dispose();
    const geometries = new Set<BufferGeometry>();
    const materials = new Set<Material>();
    this.group.traverse((object) => {
      if (object instanceof Light) object.dispose();
      if (!(object instanceof Mesh) || object.userData.sharedStructure) return;
      geometries.add(object.geometry);
      if (Array.isArray(object.material)) object.material.forEach((material) => materials.add(material));
      else materials.add(object.material);
    });
    for (const run of this.wallRuns) {
      Object.values(run).forEach((material) => materials.add(material));
    }
    geometries.forEach((geometry) => geometry.dispose());
    materials.forEach(disposeMaterial);
    this.group.removeFromParent();
    this.group.clear();
  }
}
