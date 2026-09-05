import type { BrowserObserver } from "./observer";

export interface SourceBinding { role: string; path: string; sha256: string }
export const digest = async (bytes: Uint8Array<ArrayBuffer>) => Array.from(
  new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), byte => byte.toString(16).padStart(2, "0")).join("");
export const jsonBytes = (value: unknown) => new TextEncoder().encode(JSON.stringify(value));

export async function capture(observer: BrowserObserver, route: "live" | "replay", sources: SourceBinding[]) {
  const view = observer.view, snapshot = view.snapshot;
  if (!snapshot || snapshot !== observer.state.snapshot) throw new Error("no matching authoritative presentation");
  // Copy every surface synchronously from one presentation, before hashing yields.
  const recording = jsonBytes({ schema_version: 1, kind: "browser_observer_recording", envelopes: observer.recorded() });
  const pgm = view.identityRaster();
  const image = Uint8Array.from(atob(view.canvas.toDataURL("image/png").split(",")[1]!), char => char.charCodeAt(0));
  const targets = structuredClone(view.targets), camera = structuredClone(view.cameraIdentity);
  const frame = snapshot.envelope.frame;
  const sourceBindings = structuredClone(sources);
  const sidecar = {
    schema_version: 1, kind: "capture_identity_sidecar", producer: "browser_authoritative_view", route,
    frame_generation: snapshot.generation, camera, targets,
    viewport: { width: view.width, height: view.height },
    scene: { realm: frame.observation_center.realm, level: frame.observation_center.level,
      logical_time: frame.logical_time, observation_center: frame.observation_center.position },
    image: { path: "capture.png", sha256: await digest(image) },
    identity_raster: { path: "capture.identity.pgm", format: "pgm_p5_u16_be_target_index", width: view.width, height: view.height, sha256: await digest(pgm) },
    authority: { path: "capture.frame.json", sha256: await digest(recording),
      envelope_sha256: await digest(jsonBytes(snapshot.envelope)), server_sequence: snapshot.envelope.server_sequence,
      world_revision: snapshot.envelope.world_revision, sources: sourceBindings },
  };
  return { image: Array.from(image), raster: Array.from(pgm), recording: Array.from(recording), sidecar };
}
