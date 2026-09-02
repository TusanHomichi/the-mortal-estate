import {
  AmbientLight,
  BufferAttribute,
  BufferGeometry,
  Color,
  DirectionalLight,
  DoubleSide,
  DynamicDrawUsage,
  Group,
  InstancedMesh,
  Matrix4,
  Mesh,
  MeshBasicMaterial,
  MeshStandardMaterial,
  Object3D,
  PlaneGeometry,
  PointLight,
  ShaderMaterial,
  ShadowMaterial,
  Vector3,
  type Material,
  type OrthographicCamera,
} from "three";
import { projectedHeightCoverTiles } from "../camera";
import type { FeelSpace, PropPlacement, WallRun } from "../feelTypes";
import { buildGroundGeometry } from "../groundGeometry";
import type { Preset } from "../presets";
import {
  buildRoofGeometry,
  mergeGeometryData,
  type RoofMaterial,
} from "../roofGeometry";
import {
  groundFragmentShader,
  groundVertexShader,
  swayFragmentShader,
  swayVertexShader,
} from "../shaders";
import { buildWallProfile, WALL_PROFILE, type WallMaterial } from "../wallGeometry";
import type { Cell } from "../walk/layoutPassability";
import { occludingRuns } from "../walk/wallOcclusion";
import { nearWallRunIndices } from "./interiorWalls";
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

export interface CaretakerObjects {
  card: Mesh;
  contactShadow: Mesh;
}

export interface SpaceSceneOptions {
  name: string;
  space: FeelSpace;
  textures: Map<string, DecodedTexture>;
  presets: readonly Preset[];
  anisotropy: number;
  camera: OrthographicCamera;
  caretakerCell: Cell;
  caretakerFacing: 1 | -1;
}

const WARM_LIGHT = new Color("#ffb457");
const CARETAKER_NOMINAL_HEIGHT = 1.38;
const RAIN_COUNT = 1080;
const WALL_FADE_DURATION_SECONDS = 0.35;
const WALL_FADED_PLASTER_OPACITY = 0.34;
const WALL_FADED_TIMBER_OPACITY = 0.48;
const WALL_FADED_RENDER_ORDER = 10;
const WALL_COVER_TILES = projectedHeightCoverTiles(WALL_PROFILE.capTop);

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
  readonly caretaker: CaretakerObjects;
  readonly background: Color;
  readonly weatherEnabled: boolean;

  private readonly palette: ScenePalette;
  private readonly swayMaterials: ShaderMaterial[] = [];
  private readonly wallRuns: WallRunPresentation[] = [];
  private readonly keyLight: DirectionalLight;
  private readonly keyTarget: Object3D;
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
    this.background = this.palette.background;
    this.addGround();
    this.addWalls();
    this.addRoofs();
    const lights = this.addLights(caretakerCell);
    this.keyLight = lights.key;
    this.keyTarget = lights.target;
    this.lantern = lights.lantern;
    this.lanternBase = lights.lanternBase;
    this.caretaker = this.addProps();
    this.caretaker.card.scale.x = options.caretakerFacing * Math.abs(this.caretaker.card.scale.x);
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
      const material = new MeshStandardMaterial({
        name: `roof-${key}`,
        map,
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

  private addLights(focusCell: Cell): {
    key: DirectionalLight;
    target: Object3D;
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
      lantern,
      lanternBase: this.palette.lanternIntensity,
    };
  }

  private addProps(): CaretakerObjects {
    const { space, textures, anisotropy, caretakerCell } = this.options;
    let caretaker: CaretakerObjects | null = null;
    const placements: PropPlacement[] = [
      ...space.props,
      {
        kind: "caretaker",
        cell_anchor: [caretakerCell.i, caretakerCell.j],
        nominal_height: CARETAKER_NOMINAL_HEIGHT,
        sway: false,
        mirror: false,
        facing: "view",
      },
    ];
    const lanternPosition = space.light_sources.lantern_glass === null
      ? new Vector3(0, 0, 0)
      : new Vector3().fromArray(space.light_sources.lantern_glass);
    for (const prop of placements) {
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
              windStrength: {
                value: space.weather && this.options.presets.includes("wind") ? 1 : 0.12,
              },
              timeOffset: { value: prop.cell_anchor[0] * 0.73 + prop.cell_anchor[1] * 1.13 },
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
      if (material instanceof ShaderMaterial) this.swayMaterials.push(material);
      const mesh = new Mesh(geometry, material);
      mesh.name = `Prop_${prop.kind}`;
      const transform = propCardTransform(prop);
      mesh.scale.x = transform.scaleX;
      mesh.position.set(transform.position.x, transform.position.y, transform.position.z);
      mesh.rotation.set(0, transform.rotationY, 0);
      mesh.castShadow = true;
      const contactShadow = addContactShadow(
        this.group,
        transform.position.x,
        transform.position.z,
        prop.nominal_height,
      );
      const shadowRotation = transform.contactShadowRotation;
      contactShadow.rotation.set(
        shadowRotation.x,
        shadowRotation.y,
        shadowRotation.z,
        shadowRotation.order,
      );
      this.group.add(mesh);
      if (prop.kind === "caretaker") caretaker = { card: mesh, contactShadow };
      if (prop.kind === "hearth") {
        const hearthLight = new PointLight(
          WARM_LIGHT,
          this.palette.candleIntensity * 3,
          3.4,
          2,
        );
        hearthLight.name = "HearthGlow";
        hearthLight.position.set(prop.cell_anchor[0], 0.55, prop.cell_anchor[1]);
        this.group.add(hearthLight);
      }
    }
    if (caretaker === null) throw new Error("the space scene failed to place its caretaker");
    return caretaker;
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

  update(elapsed: number): void {
    const noise = Math.sin(elapsed * 5.7 + 1.731) * 0.055 +
      Math.sin(elapsed * 11.3 + 2.943) * 0.025;
    if (this.lantern !== null) this.lantern.intensity = this.lanternBase * (1 + noise);
    for (const material of this.swayMaterials) {
      material.uniforms.elapsed!.value = elapsed;
      material.uniforms.lanternStrength!.value =
        this.palette.practicalShaderStrength * (1 + noise);
    }
    this.rain?.update(elapsed);
  }

  dispose(): void {
    const geometries = new Set<BufferGeometry>();
    const materials = new Set<Material>();
    this.group.traverse((object) => {
      if (!(object instanceof Mesh)) return;
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
