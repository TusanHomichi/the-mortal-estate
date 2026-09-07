import type { Position } from "../authoritative/state";

/** Read-only fields from the Rust-validated preview response. */
export interface PathPreview {
  actor_id: string; start: Position; requested_path: string[];
  accepted_steps: string; final_position: Position;
  steps: { attempted: Position; outcome: { kind: string } }[];
}
export interface PathPreviewResult {
  kind: "path_preview_result"; preview_id: string;
  disposition: { kind: string }; control_epoch: string;
  actor_id: string; world_revision: string; preview: PathPreview | null;
}
