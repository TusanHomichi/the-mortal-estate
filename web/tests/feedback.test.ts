import { describe, it, expect } from "vitest";
import { feedbackText } from "../src/play/feedback";

describe("private reply presentation", () => {
  it("presents typed replies without inventing a missing critique rank", () => {
    expect(feedbackText([{ kind: "feedback", cue: { kind: "skill_critique", track_id: "staff", track_display: "Staff", level: 0, critique_rank: null, level_title: null } }])).toEqual(["Staff: level 0."]);
    expect(feedbackText([{ kind: "feedback", cue: { kind: "npc_message", npc_name: "Tomas", response: "Bring the fallen here." } }])).toEqual(["Tomas: Bring the fallen here."]);
    expect(feedbackText([{ kind: "actor_moved" }])).toEqual([]);
  });
});
