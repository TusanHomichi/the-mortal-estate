// Add only the proof authority; restore the user's Chromium trust after launch
// lifetime. Firefox uses a disposable profile. No certificate-error bypass.
import { execFileSync } from "node:child_process";
import { access, mkdir, mkdtemp, rm } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import os from "node:os";
import path from "node:path";

export async function trustAuthority(name, authority) {
  if (process.platform !== "linux") throw new Error("Trusted local browser proof currently requires Linux NSS tooling");
  let profile;
  let database;
  if (name === "firefox") {
    profile = await mkdtemp(path.join(os.tmpdir(), "tme-trusted-firefox-")); database = profile;
  } else {
    const legacy = path.join(os.homedir(), ".pki/nssdb");
    try { await access(legacy); database = legacy; }
    catch { database = path.join(os.homedir(), ".local/share/pki/nssdb"); }
  }
  const nickname = `tme-private-proof-${randomUUID()}`;
  let added = false;
  const stop = async () => {
    try { if (added) execFileSync("certutil", ["-D", "-d", `sql:${database}`, "-n", nickname], { stdio: "pipe" }); }
    finally { if (profile) await rm(profile, { recursive: true, force: true }); }
  };
  try {
    await mkdir(database, { recursive: true, mode: 0o700 });
    try { await access(path.join(database, "cert9.db")); }
    catch { execFileSync("certutil", ["-N", "--empty-password", "-d", `sql:${database}`], { stdio: "pipe" }); }
    execFileSync("certutil", ["-A", "-d", `sql:${database}`, "-n", nickname, "-t", "C,,", "-i", authority], { stdio: "pipe" });
    added = true;
    return { profile, stop };
  } catch (error) { await stop(); throw error; }
}
