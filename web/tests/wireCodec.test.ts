import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import { beforeAll, describe, expect, it } from "vitest";
import { WireCodec } from "../src/authoritative/codec";
import { AuthoritativeState } from "../src/authoritative/state";

const root = path.resolve(import.meta.dirname, "../..");
let codec: WireCodec;
beforeAll(async () => {
  const target = path.resolve(root, process.env.CARGO_TARGET_DIR || "target");
  codec = await WireCodec.create(readFileSync(path.join(target, "wasm32-unknown-unknown/release/tme_protocol.wasm")));
});
const corpus = path.join(root, "tests/fixtures/wire");
const fixtures = readdirSync(corpus).sort().map(name => JSON.parse(readFileSync(path.join(corpus, name), "utf8")));
describe("Rust WebAssembly shared wire corpus", () => {
  for (const fixture of fixtures) for (const row of fixture.cases) {
    it(`${fixture.decoder}: ${row.case_id}`, () => {
      const input = row.input_utf8 === undefined ? Buffer.from(row.input_hex, "hex") : row.input_utf8;
      const decode = () => codec.decode(fixture.decoder, input);
      if (row.expect === "accept") expect(decode).not.toThrow();
      else expect(decode).toThrow();
    });
  }
});

const welcome = JSON.parse(fixtures.find(row => row.decoder === "server_envelope").cases
  .find((row: { expect: string; input_utf8?: string }) => row.expect === "accept" && row.input_utf8?.includes('"server_welcome"')).input_utf8);
describe("atomic authoritative frame replacement", () => {
  it("requires a welcome and refuses regressed/conflicting sequences without changing state", () => {
    const state = new AuthoritativeState(codec);
    const sample = JSON.parse(fixtures.find(row => row.decoder === "server_envelope").cases
      .find((row: { expect: string; input_utf8?: string }) => row.expect === "accept" && row.input_utf8?.includes('"state_update"')).input_utf8);
    expect(() => state.accept(JSON.stringify(sample))).toThrow("before welcome");
    state.accept(JSON.stringify(welcome));
    const first = state.snapshot;
    expect(state.accept(JSON.stringify({ ...sample, server_sequence: welcome.server_sequence,
      world_revision: welcome.world_revision, frame: welcome.frame, static_scene_context: welcome.static_scene_context }))).toBe(false);
    expect(state.accept(JSON.stringify({ ...sample, server_sequence: (BigInt(welcome.server_sequence) + 7n).toString(), world_revision: welcome.world_revision }))).toBe(true);
    const newer = state.snapshot;
    expect(newer).not.toBe(first);
    expect(() => state.accept(JSON.stringify(sample))).toThrow();
    expect(state.snapshot).toBe(newer);
    expect(state.accept(newer!.raw)).toBe(false);
    expect(() => state.accept(JSON.stringify({ ...newer!.envelope, world_revision: (BigInt(welcome.world_revision) + 1n).toString() }))).toThrow("conflicting");
    expect(Object.isFrozen(newer!.envelope.frame)).toBe(true);
    state.reset();
    expect(state.snapshot).toBeNull();
    state.accept(JSON.stringify(welcome));
    expect(state.snapshot!.generation).toBeGreaterThan(newer!.generation);
  });
});
