/** Presentation of typed, Rust-decoded private feedback. No raw event fallback. */
type Cue = { kind: "skill_critique"; track_id: string; track_display: string | null; level: number; critique_rank: number | null; level_title: string | null }
  | { kind: "npc_message"; npc_name: string; response: string };
export interface FeedbackEvent { kind: string; cue?: Cue }
export function feedbackText(events: readonly FeedbackEvent[]): string[] {
  return events.flatMap(event => {
    if (event.kind !== "feedback" || !event.cue) return [];
    const cue = event.cue;
    if (cue.kind === "npc_message") return [`${cue.npc_name}: ${cue.response}`];
    if (cue.kind === "skill_critique") return [`${cue.track_display ?? cue.track_id}: ${cue.level_title ?? `level ${cue.level}`}${cue.critique_rank === null ? "" : ` · rank ${cue.critique_rank}`}.`];
    return [];
  });
}
