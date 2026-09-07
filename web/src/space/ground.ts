import { BufferAttribute, BufferGeometry, Color, Group, Mesh, PlaneGeometry, ShaderMaterial, ShadowMaterial, Vector3 } from "three";
import { buildGroundGeometry } from "../groundGeometry";
import { groundFragmentShader, groundVertexShader } from "../shaders";
import type { TerrainSurface } from "../terrainSurface";
import { addCoastalGround } from "./coastalGround";
import type { SpaceSceneOptions } from "./SpaceScene";
import type { ScenePalette } from "./palette";
import { configureTexture, requiredTexture } from "./textures";

export function addGround(group: Group, options: SpaceSceneOptions, palette: ScenePalette, elapsed: { value: number }, surface: TerrainSurface): void {
    const { space, textures, presets, anisotropy } = options;
    if (surface.coastal) { addCoastalGround(group, options, palette, elapsed, surface); return; }
    const rainy = space.weather && presets.includes("rain");
    const ambient = palette.ambient.clone().multiplyScalar(palette.ambientIntensity);
    const key = palette.key.clone().multiplyScalar(palette.keyIntensity * 0.44);
    const cellsByMaterial = new Map<string, typeof space.cells>();
    for (const cell of space.cells) {
      if (cell.material === "void") continue;
      const cells = cellsByMaterial.get(cell.material) ?? [];
      cells.push(cell);
      cellsByMaterial.set(cell.material, cells);
    }
    for (const [materialName, cells] of cellsByMaterial) {
      const water = materialName === "water";
      const swatch = requiredTexture(textures, `terrain/${materialName}`).texture;
      configureTexture(swatch, anisotropy);
      const material = new ShaderMaterial({
        name: `ground-${materialName}`,
        uniforms: {
          swatch: { value: swatch },
          swatchPeriod: { value: water ? 6 : 3 },
          waterSurface: { value: water ? 1 : 0 },
          elapsed: elapsed,
          wetness: { value: rainy ? 1 : 0 },
          timeTint: {
            value: water
              ? new Color(0.48, 0.6, 0.64)
              : materialName === "lane"
                ? new Color(0.6, 0.58, 0.5)
                : space.weather && presets.includes("dusk")
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
      group.add(mesh);
      if (water && space.weather) {
        // Scenic sea beyond the bounded candidate; never a walkable cell.
        const sea = new Mesh(new PlaneGeometry(400, 400), material);
        sea.name = "SeaBackdrop";
        sea.rotation.x = -Math.PI / 2;
        sea.position.set(space.grid_extents.i / 2, -0.025, space.grid_extents.j / 2);
        group.add(sea);
      }
    }

    // A room-wide shadow plane would write depth across stair openings and
    // hide geometry below the floor, even where its colour is transparent.
    const shadowData = buildGroundGeometry(space.cells.filter(cell => cell.material !== "void"));
    const shadowGeometry = new BufferGeometry();
    shadowGeometry.setAttribute("position", new BufferAttribute(new Float32Array(shadowData.positions), 3));
    shadowGeometry.setIndex(shadowData.indices);
    shadowGeometry.computeVertexNormals();
    const shadowPlane = new Mesh(
      shadowGeometry,
      new ShadowMaterial({ color: 0x02050b, opacity: 0.34 }),
    );
    shadowPlane.name = "GroundShadowReceiver";
    shadowPlane.position.y = -0.003;
    shadowPlane.receiveShadow = true;
    group.add(shadowPlane);
  }
