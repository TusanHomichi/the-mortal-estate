import type { Coord } from "../authoritative/state";
import type { Cell } from "../walk/layoutPassability";
import type { ControlView } from "./control";
import type { PathPreview } from "./pathPreview";
import { proposePath, routeDirections } from "./pathPlan";

export interface WalkPresentation { route: readonly Cell[] | null; kind: "draft" | "committed"; hover: Coord | null; cursor: "ready" | "waiting" | "refused" }
interface Transport {
  previewPath(path: readonly string[]): Promise<PathPreview | null>;
  cancelPathPreview(): void;
  command(intent: unknown): boolean;
}
const same = (a: Coord | null, b: Coord | null) => a?.x === b?.x && a?.y === b?.y;

/** Discardable input intent. Position, outcomes and readiness stay on server. */
export class PathControls {
  private state: ControlView | null = null;
  private identity = "";
  private spaceIdentity = "";
  private draft: Cell[] | null = null;
  private committed: Cell[] | null = null;
  private target: Coord | null = null;
  private hover: Coord | null = null;
  private serial = 0;
  private checking = false;
  private confirm = false;
  private refused = false;
  constructor(private readonly transport: Transport, private readonly draw: (view: WalkPresentation) => void) {}
  private get ready(): boolean {
    return this.state?.phase === "playing" && !this.state.busy && !this.state.pending && !!this.state.snapshot?.envelope.frame.can_act;
  }
  present(state: ControlView): void {
    this.state = state;
    const frame = state.snapshot?.envelope.frame;
    const identity = frame ? JSON.stringify([frame.observer_actor_id, frame.observation_center]) : "";
    const spaceIdentity = frame ? JSON.stringify([frame.observer_actor_id,frame.observation_center.realm,frame.observation_center.level]) : "";
    if (this.spaceIdentity !== spaceIdentity) {
      this.committed = null; this.hover = null; this.spaceIdentity = spaceIdentity;
    }
    if (this.identity !== identity || state.phase !== "playing" || !this.ready) {
      this.clearDraft(); this.identity = identity;
    }
    if (this.ready || !frame || state.phase !== "playing") this.committed = null;
    this.render();
  }
  hoverAt(target: Coord | null): void { this.hover = target; this.refused = false; this.render(); }
  cancel(): void { this.clearDraft(); this.render(); }
  private clearDraft(): void {
    ++this.serial; this.transport.cancelPathPreview();
    this.draft = null; this.target = null; this.checking = false; this.confirm = false; this.refused = false;
  }
  click(target: Coord | null): void {
    if (!this.ready) return;
    if (target && same(target, this.target)) {
      if (this.confirm) return;
      this.confirm = true;
      if (!this.checking) void this.assess(true);
      return;
    }
    this.clearDraft();
    const frame = this.state!.snapshot!.envelope.frame;
    this.draft = target && proposePath(frame, target);
    this.target = this.draft ? target : null;
    this.refused = !this.draft;
    this.render();
    if (this.draft) void this.assess(false);
  }
  private async assess(commit: boolean): Promise<void> {
    if (!this.draft || !this.ready) return;
    const serial = this.serial, route = this.draft, path = routeDirections(route);
    this.checking = true;
    let preview: PathPreview | null = null;
    try { preview = await this.transport.previewPath(path); } catch { /* Remain an uncommitted refusal. */ }
    if (serial !== this.serial) return;
    this.checking = false;
    // Partial movement is not silently substituted for the endpoint confirmed.
    const accepted = preview && BigInt(preview.accepted_steps) === BigInt(path.length) &&
      preview.steps.every(step => step.outcome.kind !== "blocked");
    if (!this.ready || !accepted) { this.clearDraft(); this.refused = true; this.render(); return; }
    if (commit || this.confirm) {
      this.draft = null; this.target = null; this.confirm = false;
      this.committed = route;
      // Draw before synchronous control emission so the renderer can bind the
      // traversed prefix to the authoritative landing, including partial moves.
      this.render();
      if (!this.transport.command({ kind: "move_path", path })) this.committed = null;
    }
    this.render();
  }
  private render(): void {
    const frame = this.state?.snapshot?.envelope.frame;
    const unavailable = !this.ready;
    const refused = this.refused || (!!this.hover && !!frame && !proposePath(frame,this.hover));
    this.draw({ route: this.committed ?? this.draft, kind: this.committed ? "committed" : "draft",
      hover: this.hover, cursor: unavailable ? "waiting" : refused ? "refused" : "ready" });
  }
}
