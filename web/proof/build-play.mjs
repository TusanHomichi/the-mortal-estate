// Build the private authoritative client from carried source and the Rust codec.
import { spawnSync } from "node:child_process";
import { copyFile, mkdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build } from "vite";
const root = fileURLToPath(new URL("../../", import.meta.url));
const web = path.join(root, "web");
const output = path.resolve(process.argv[2] || path.join(web, "dist/play"));
const result = spawnSync(process.execPath, [path.join(web, "proof/build-codec.mjs")], { cwd: root, stdio: "inherit" });
if (result.status !== 0) throw new Error("Rust codec build failed");
await build({ configFile: false, root: web, build: { outDir: output, emptyOutDir: true,
  rollupOptions: { input: path.join(web, "play.html") } } });
await mkdir(output, { recursive: true });
const target = path.resolve(root, process.env.CARGO_TARGET_DIR || "target");
await copyFile(path.join(target, "wasm32-unknown-unknown/release/tme_protocol.wasm"), path.join(output, "codec.wasm"));
