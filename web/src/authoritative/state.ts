import type { WireCodec } from "./codec";

// Read-only presentation types. Rust validates the complete envelope, including
// fields this diagnostic renderer does not use; these are not a wire decoder.
export interface Coord { x: number; y: number }
export interface Position { realm: string; level: string; position: Coord }
export interface Frame {
  logical_time: string; ready_at: string; can_act: boolean;
  observer_actor_id: string; observation_center: Position;
  tiles: { position: Coord; terrain_id?: string }[];
  actors: { actor_id: string; position: Position; name: string }[];
  corpses: { corpse_id: string; location: Position }[];
  ground_items: { item_instance_id: string; location: Position }[];
  gold_piles: { gold_pile_id: string; location: Position }[];
}
export interface Envelope {
  kind: string;
  server_sequence: string;
  world_revision: string;
  control_epoch?: string;
  frame: Frame;
  static_scene_context: unknown;
}
export interface Snapshot {
  readonly generation: number;
  readonly envelope: Envelope;
  readonly raw: string;
}

function freeze<T>(value: T): T {
  if (value && typeof value === "object") {
    Object.values(value).forEach(freeze);
    Object.freeze(value);
  }
  return value;
}

export class AuthoritativeState {
  snapshot: Snapshot | null = null;
  private generation = 0;
  constructor(private readonly codec: WireCodec) {}

  reset(): void { this.snapshot = null; }

  accept(raw: string): boolean {
    const envelope = this.codec.decode<Envelope>("server_envelope", raw);
    if (envelope.kind !== "server_welcome" && envelope.kind !== "state_update") return false;
    const previous = this.snapshot;
    if (!previous && envelope.kind !== "server_welcome") throw new Error("update before welcome");
    if (previous && envelope.kind === "server_welcome") throw new Error("unexpected welcome in active connection");
    if (previous) {
      const sequence = BigInt(envelope.server_sequence);
      const before = BigInt(previous.envelope.server_sequence);
      if (sequence < before) throw new Error("server sequence regressed");
      if (sequence === before) {
        const stateBytes = (value: Envelope) => JSON.stringify({ world_revision: value.world_revision,
          frame: value.frame, static_scene_context: value.static_scene_context });
        if (stateBytes(envelope) === stateBytes(previous.envelope)) return false;
        throw new Error("conflicting state at the same sequence");
      }
      if (BigInt(envelope.world_revision) < BigInt(previous.envelope.world_revision)) {
        throw new Error("world revision regressed");
      }
    }
    this.snapshot = freeze({ generation: ++this.generation, envelope, raw });
    return true;
  }
}
