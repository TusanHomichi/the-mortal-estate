// Rebuild from carried Rust sources; an ignored binary never supplies authority.
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
const root = fileURLToPath(new URL("../../", import.meta.url));
const result = spawnSync("cargo", ["build", "-p", "tme-protocol", "--target", "wasm32-unknown-unknown", "--release", "--locked"], {
  cwd: root, stdio: "inherit",
});
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
