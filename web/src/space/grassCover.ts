import { BufferAttribute, BufferGeometry, Color, DoubleSide, Group, InstancedMesh, MeshDepthMaterial, MeshStandardMaterial, Object3D, RGBADepthPacking, Vector2, Vector3 } from "three";
import type { FeelSpace } from "../feelTypes";
import type { TerrainSurface } from "../terrainSurface";
import { pathCover } from "./groundCover";

export interface GrassCover { count: number; update(elapsed: number, player: Vector3): void; dispose(): void }
/** Geometry blades: their colour and shadow passes use the same rooted bend. */
export function addGrassCover(group: Group, space: FeelSpace, surface: TerrainSurface,
  wind: { elapsed: { value: number }; windDirection: { value: Vector2 }; windStrength: { value: number } }): GrassCover {
  const placements: { x: number; z: number; h: number; yaw: number }[] = [];
  let seed = 905;
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
  for (const c of space.cells) {
    if (c.material !== "grass" && c.material !== "meadow" && c.material !== "lane") continue;
    for (let z = 0; z < 6; z++) for (let x = 0; x < 6; x++) {
      const px = c.i - .5 + (x + .12 + random() * .76) / 6;
      const pz = c.j - .5 + (z + .12 + random() * .76) / 6;
      const sample = surface.sample(px, pz);
      const wear = pathCover(sample, px, pz);
      if (sample.land < .84 || wear > .53) continue;
      const edgeHeight = 1 - Math.max(0, wear - .18) * 1.3;
      placements.push({ x: px, z: pz, h: (c.material === "meadow" ? .31 : .075) * (.9 + random() * .2) * edgeHeight, yaw: random() * Math.PI * 2 });
    }
  }
  const positions: number[] = [], colours: number[] = [], indices: number[] = [];
  for (let blade = 0; blade < 7; blade++) {
    const angle = blade * 2.4, x = Math.cos(angle) * .09, z = Math.sin(angle) * .09;
    const dx = Math.cos(angle) * .013, dz = Math.sin(angle) * .013, start = positions.length / 3;
    const bendX = Math.sin(angle) * .065, bendZ = -Math.cos(angle) * .065;
    positions.push(x-dx*.5,0,z-dz*.5, x+dx*.5,0,z+dz*.5,
      x+dx*.75+bendX*.4,.6,z+dz*.75+bendZ*.4,
      x-dx*.75+bendX*.4,.6,z-dz*.75+bendZ*.4, x+bendX,1,z+bendZ);
    for (const shade of [.62,.62,.88,.88,1]) colours.push(shade,shade,shade);
    indices.push(start,start+1,start+2,start,start+2,start+3,start+3,start+2,start+4);
  }
  const geometry = new BufferGeometry(); geometry.setAttribute("position", new BufferAttribute(new Float32Array(positions),3));
  geometry.setAttribute("color", new BufferAttribute(new Float32Array(colours),3)); geometry.setIndex(indices); geometry.computeVertexNormals();
  const material = new MeshStandardMaterial({ name: "LivingGrass", color: new Color("#899557"), roughness: 1, side: DoubleSide, vertexColors: true });
  const flowerMaterial = new MeshStandardMaterial({ name: "MeadowFlowers", roughness: 1, side: DoubleSide, vertexColors: true });
  const depth = new MeshDepthMaterial({ depthPacking: RGBADepthPacking, side: DoubleSide });
  const trail = Array.from({length: 8}, () => new Vector3(-1000,-1000,-1000));
  const uniforms = { ...wind, grassTrail: { value: trail } };
  const declarations = `uniform float elapsed; uniform vec2 windDirection; uniform float windStrength; uniform vec3 grassTrail[8];`;
  const bend = `
    vec3 transformed = vec3(position);
    vec3 anchor = (modelMatrix * instanceMatrix * vec4(0.,0.,0.,1.)).xyz;
    float rootWeight = position.y * position.y;
    vec2 displacement = windDirection * sin(anchor.x*1.3 + anchor.z*.8 - elapsed*1.7) * windStrength * .045;
    for (int n=0; n<8; n++) {
      vec2 away = anchor.xz - grassTrail[n].xy;
      float distanceToStep = length(away);
      float recovery = clamp(1.0 - (elapsed - grassTrail[n].z) / 2.8, 0., 1.);
      float pressure = (1.0 - smoothstep(.15, .7, distanceToStep)) * recovery;
      displacement += away / max(distanceToStep,.08) * pressure * .055;
      transformed.y *= 1.0 - pressure * .065 * rootWeight;
    }
    // Transform the world bend to each instance's local horizontal axes.
    vec2 localBend = vec2(dot(displacement, instanceMatrix[0].xz), dot(displacement, instanceMatrix[2].xz));
    transformed.xz += localBend * rootWeight;
  `;
  for (const m of [material, flowerMaterial, depth]) {
    m.onBeforeCompile = shader => {
      Object.assign(shader.uniforms, uniforms);
      if (!shader.vertexShader.includes("#include <begin_vertex>")) throw new Error("grass shader has no bend insertion anchor");
      shader.vertexShader = declarations + "\n" + shader.vertexShader.replace("#include <begin_vertex>", bend);
    };
    m.customProgramCacheKey = () => "rooted-grass-trail-v1";
  }
  const dummy = new Object3D();
  // A single whole-land instance batch cannot be culled per patch. Small groups
  // retain every placement while letting each camera/shadow pass skip distant cover.
  function batches(name: string, geometry: BufferGeometry, material: MeshStandardMaterial,
    items: typeof placements, heightScale = 1): void {
    const root = new Group(); root.name = name; group.add(root);
    geometry.computeBoundingBox();
    const tipWeight = geometry.boundingBox!.max.y ** 2;
    const chunks = new Map<string, typeof placements>();
    for (const p of items) {
      const key = `${Math.floor(p.x / 4)},${Math.floor(p.z / 4)}`;
      const chunk = chunks.get(key) ?? []; chunk.push(p); chunks.set(key, chunk);
    }
    for (const [key, items] of chunks) {
      const mesh = new InstancedMesh(geometry, material, items.length);
      mesh.name = `${name}_${key}`; mesh.customDepthMaterial = depth;
      mesh.castShadow = true; mesh.receiveShadow = true;
      items.forEach((p, index) => {
        dummy.position.set(p.x, surface.heightAt(p.x,p.z), p.z); dummy.rotation.y = p.yaw;
        dummy.scale.set(1,p.h*heightScale,1); dummy.updateMatrix(); mesh.setMatrixAt(index,dummy.matrix);
      });
      mesh.instanceMatrix.needsUpdate = true; mesh.computeBoundingSphere();
      // Conservative maximum wind + eight recent steps at this geometry's tip.
      mesh.boundingSphere!.radius += (Math.abs(wind.windStrength.value) * .045 + trail.length * .055) * tipWeight;
      root.add(mesh);
    }
  }
  batches("GrassCover", geometry, material, placements);
  // Sparse cream flowers occupy the same rooted cover, leaving worn ground clear.
  const flowers = placements.filter((p, i) => i % 47 === 0 && p.h > .20);
  const fp: number[] = [], fc: number[] = [], fi: number[] = [];
  for (let stem = 0; stem < 3; stem++) {
    const x = (stem - 1) * .065, z = (stem % 2) * .045, top = 1 + stem * .10;
    let start = fp.length / 3;
    fp.push(x-.006,0,z, x+.006,0,z, x+.025,top,z, x+.014,top,z);
    for (let n = 0; n < 4; n++) fc.push(.21,.29,.085);
    fi.push(start,start+1,start+2,start,start+2,start+3);
    for (let petal = 0; petal < 5; petal++) {
      const a = petal * Math.PI * 2 / 5, b = a + Math.PI * 2 / 5;
      start = fp.length / 3;
      fp.push(x+.02,top,z, x+.02+Math.cos(a)*.034,top+.03,z+Math.sin(a)*.034,
        x+.02+Math.cos(b)*.034,top+.03,z+Math.sin(b)*.034);
      fc.push(.57,.40,.10, .83,.78,.54, .83,.78,.54);
      fi.push(start,start+1,start+2);
    }
  }
  const flowerGeometry = new BufferGeometry();
  flowerGeometry.setAttribute("position", new BufferAttribute(new Float32Array(fp), 3));
  flowerGeometry.setAttribute("color", new BufferAttribute(new Float32Array(fc), 3));
  flowerGeometry.setIndex(fi); flowerGeometry.computeVertexNormals();
  batches("MeadowFlowers", flowerGeometry, flowerMaterial, flowers, 1.15);
  let cursor = 0, lastStep = -1;
  return { count: placements.length,
    update(elapsed, player) {
      if (elapsed-lastStep < .12) return;
      trail[cursor]!.set(player.x,player.z,elapsed); cursor=(cursor+1)%trail.length; lastStep=elapsed;
    },
    dispose() {
      depth.dispose();
      // With no placements there is no mesh for SpaceScene's shared-resource
      // disposal traversal to find; only those unreferenced resources live here.
      if (!placements.length) { geometry.dispose(); material.dispose(); }
      if (!flowers.length) { flowerGeometry.dispose(); flowerMaterial.dispose(); }
    },
  };
}
