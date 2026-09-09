import { Color, Float32BufferAttribute, Group, IcosahedronGeometry, InstancedMesh, MeshStandardMaterial, Object3D, Vector3 } from "three";
import type { FeelSpace } from "../feelTypes";
import { SEA_HEIGHT, type TerrainSurface } from "../terrainSurface";

/** Small embedded stones give path margins and banks a broken surface.
 * Decorative only: neither instances nor their bounds define occupancy.
 */
export function addGroundStones(group: Group, space: FeelSpace, surface: TerrainSurface): void {
  let seed = 1811;
  const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
  const waterMargins = new Set<string>();
  for (const cell of space.cells) if (cell.material === "water") {
    for (let dz = -1; dz <= 1; dz++) for (let dx = -1; dx <= 1; dx++) {
      waterMargins.add(`${cell.i + dx},${cell.j + dz}`);
    }
  }
  const stones: { x: number; y: number; z: number; r: number; height: number; yaw: number; tone: number }[] = [];
  for (const cell of space.cells) {
    if (!["grass", "meadow", "lane", "water"].includes(cell.material)) continue;
    const nearWater = waterMargins.has(`${cell.i},${cell.j}`);
    const patch = random();
    for (let i = 0; i < (nearWater ? 12 : 5); i++) {
      const x = cell.i + (random() - .5) * .92, z = cell.j + (random() - .5) * .92;
      const s = surface.sample(x, z, cell.material);
      if (s.height < SEA_HEIGHT + .035 || s.land < .58) continue;
      if (space.portals.some(p => Math.hypot(x - p.cell[0], z - p.cell[1]) < 1)) continue;
      const bank = nearWater && (s.land < .9 || ([[.24, 0], [-.24, 0], [0, .24], [0, -.24]] as const)
        .some(([dx, dz]) => surface.sample(x + dx, z + dz, cell.material).land < .68));
      if (!bank && (patch < .7 || random() < .7)) continue;
      const bankGroup = .5 + .5 * Math.sin(x * 2.3 + Math.sin(z * 1.7) * 1.8);
      if (bank && bankGroup < .32) continue;
      const radius = bank ? .08 + bankGroup * random() * .19 : .025 + random() * .04;
      if (cell.material === "lane" && Math.abs(x - cell.i) < .34 + radius && Math.abs(z - cell.j) < .34 + radius) continue;
      if (space.portals.some(p => Math.hypot(x - p.cell[0], z - p.cell[1]) < 1 + radius)) continue;
      stones.push({ x, y: s.height + radius * .12, z, r: radius, height: bank ? .62 : .46,
        yaw: random() * Math.PI * 2, tone: .8 + random() * .35 });
    }
  }
  if (!stones.length) return;
  const geometry = new IcosahedronGeometry(1, 0);
  // Broad face tones retain the stone planes under the ambient-only building shadows.
  const positions = geometry.getAttribute("position"), colors: number[] = [];
  const a = new Vector3(), b = new Vector3(), c = new Vector3();
  for (let i = 0; i < positions.count; i += 3) {
    a.fromBufferAttribute(positions, i); b.fromBufferAttribute(positions, i + 1); c.fromBufferAttribute(positions, i + 2);
    const normal = b.sub(a).cross(c.sub(a)).normalize();
    const tone = .9 + .65 * (-normal.x * .5 + normal.y * .65 + normal.z * .5);
    for (let v = 0; v < 3; v++) colors.push(tone, tone, tone);
  }
  geometry.setAttribute("color", new Float32BufferAttribute(colors, 3));
  const material = new MeshStandardMaterial({ name: "Bank and path stones", color: "#a29678", roughness: 1, flatShading: true, vertexColors: true });
  const mesh = new InstancedMesh(geometry, material, stones.length), dummy = new Object3D();
  stones.forEach((p, i) => {
    dummy.position.set(p.x, p.y, p.z); dummy.rotation.set(.1, p.yaw, -.14);
    dummy.scale.set(p.r, p.r * p.height, p.r * .78); dummy.updateMatrix(); mesh.setMatrixAt(i, dummy.matrix);
    mesh.setColorAt(i, new Color(p.tone, p.tone * .99, p.tone * .94));
  });
  mesh.name = "Embedded ground stones"; mesh.castShadow = true; mesh.receiveShadow = true;
  mesh.computeBoundingBox(); mesh.computeBoundingSphere(); group.add(mesh);
}
