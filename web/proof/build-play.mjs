// Build the private authoritative client from carried source and the Rust codec.
import { spawnSync } from "node:child_process";
import { copyFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const root = fileURLToPath(new URL("../../", import.meta.url));
const web = path.join(root, "web");
const output = path.resolve(process.argv[2] || path.join(web, "dist/play"));
const presentation = process.argv[3] || "world";
if (!["inspection", "world"].includes(presentation)) throw new Error("Retired or unknown play presentation build mode; use world or explicit inspection");
const result = spawnSync(process.execPath, [path.join(web, "proof/build-codec.mjs")], { cwd: root, stdio: "inherit" });
if (result.status !== 0) throw new Error("Rust codec build failed");
await build({ configFile: false, root: web,
  resolve: { alias: presentation === "inspection" ? [{ find: "./rendererFactory", replacement: path.join(web,"src/play/inspectionFactory.ts") }] : [] },
  plugins: [{ name: "world-product-boundary", generateBundle(_options,bundle) {
    if(presentation!=="world")return;
    for(const item of Object.values(bundle))if(item.type==="chunk")for(const id of Object.keys(item.modules)) {
      if(/\/play\/studyRenderer\.|\/play\/pixelDungeon|\/authoritative\/renderer\./.test(id))throw new Error(`Retired renderer reached the product: ${id}`);
    }
  } }], build: { outDir: output, emptyOutDir: true,
  rollupOptions: { input: path.join(web, "play.html") } } });
await mkdir(output, { recursive: true });
await copyFile(path.join(output,"play.html"),path.join(output,"index.html"));
const target = path.resolve(root, process.env.CARGO_TARGET_DIR || "target");
await copyFile(path.join(target, "wasm32-unknown-unknown/release/tme_protocol.wasm"), path.join(output, "codec.wasm"));
