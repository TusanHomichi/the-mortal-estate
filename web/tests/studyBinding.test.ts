import { describe, it, expect } from "vitest";
import promotion from "../../content/lands/first-expedition/promotion.json";
import { bindStudySpace, verifyStudyManifest } from "../src/play/studyBinding";
import studyReceipt from "../src/play/studyReceipt.json";
import { feedbackText } from "../src/play/feedback";
import type { Snapshot } from "../src/authoritative/state";
import type { FeelManifest } from "../src/feelTypes";

describe("authoritative presentation binding", () => {
  it("binds revised artwork to accepted geography and refuses mismatched bytes or ancestry", async () => {
    const bytes = new TextEncoder().encode(JSON.stringify({ figures: { tomas: {}, maude: {} } })).buffer;
    const receipt = { ...studyReceipt, asset_manifest_sha256: Buffer.from(await crypto.subtle.digest("SHA-256", bytes)).toString("hex") };
    await expect(verifyStudyManifest(bytes, receipt)).resolves.toBeUndefined();
    await expect(verifyStudyManifest(new Uint8Array([1]).buffer, receipt)).rejects.toThrow();
    await expect(verifyStudyManifest(bytes, { ...receipt, geography_review_manifest_sha256: "0".repeat(64) })).rejects.toThrow(/accepted geography/);
    await expect(verifyStudyManifest(bytes, { ...receipt, status: "accepted" })).rejects.toThrow(/accepted geography/);
    await expect(verifyStudyManifest(bytes, { ...receipt, schema_version: 0 })).rejects.toThrow(/accepted geography/);
  });
  const manifest = { spaces: { arrival: { grid_extents: { i: 2, j: 1 }, cells: [
    { i: 0, j: 0, walkable: true }, { i: 1, j: 0, walkable: false },
  ] } } } as unknown as FeelManifest;
  function snapshot() { return { envelope: { frame: { observation_center: { realm: "first_expedition", level: "arrival" } },
    static_scene_context: { visual_manifest_digest: promotion.master.sha256, site: { realm: "first_expedition", level: "arrival" },
      bounds: { min: { x: 0, y: 0 }, max: { x: 1, y: 0 } }, walkable_mask: [{ x: 0, y: 0 }] } } }; }
  it("refuses another world's identity, stale masks and shifted bounds", () => {
    expect(bindStudySpace(snapshot() as Snapshot, manifest)).toBe("arrival");
    const wrong = snapshot(); wrong.envelope.static_scene_context.visual_manifest_digest = "0".repeat(64);
    expect(() => bindStudySpace(wrong as Snapshot, manifest)).toThrow(/served geography/);
    const mask = snapshot(); mask.envelope.static_scene_context.walkable_mask[0]!.x = 1;
    expect(() => bindStudySpace(mask as Snapshot, manifest)).toThrow(/floor masks/);
    const bounds = snapshot(); bounds.envelope.static_scene_context.bounds.min.x = -1;
    expect(() => bindStudySpace(bounds as Snapshot, manifest)).toThrow(/bounds/);
  });
  it("presents typed private replies without inventing a missing critique rank", () => {
    expect(feedbackText([{ kind: "feedback", cue: { kind: "skill_critique", track_id: "staff", track_display: "Staff", level: 0, critique_rank: null, level_title: null } }])).toEqual(["Staff: level 0."]);
    expect(feedbackText([{ kind: "feedback", cue: { kind: "npc_message", npc_name: "Tomas", response: "Bring the fallen here." } }])).toEqual(["Tomas: Bring the fallen here."]);
    expect(feedbackText([{ kind: "actor_moved" }])).toEqual([]);
  });
});
