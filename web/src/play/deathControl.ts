import type { ControlView } from "./control";
import { observedLife } from "./life";

/** Visible death control resolves only the current authoritative character offer. */
export class DeathControl {
  private readonly notice = document.createElement("p");
  private readonly request = document.createElement("button");
  private current: { generation: number; action: string } | null = null;

  constructor(private readonly root: HTMLElement,
    private readonly dispatch: (generation: number, group: string, action: string) => boolean) {
    this.notice.setAttribute("role", "status");
    this.request.type = "button";
    this.request.onclick = () => {
      if (this.current && !this.root.hidden && !this.request.disabled && this.request.isConnected) {
        this.dispatch(this.current.generation, "character", this.current.action);
      }
    };
    this.root.append(this.notice, this.request);
  }

  present(view: ControlView): void {
    const snapshot = view.snapshot, frame = snapshot?.envelope.frame;
    const life = observedLife(frame);
    this.root.hidden = view.phase !== "playing" || !life || life === "alive";
    const message = life === "ghost"
      ? "You remain at your corpse. You can see and speak to those nearby."
      : "Your character is awaiting a return to life.";
    if (this.notice.textContent !== message) this.notice.textContent = message;
    const offer = frame?.action_options.find(action => action.intent?.kind === "request_resurrection");
    this.current = !this.root.hidden && snapshot && offer
      ? { generation: snapshot.generation, action: offer.id } : null;
    this.request.hidden = !offer;
    this.request.textContent = offer?.label ?? "";
    this.request.disabled = !this.current || view.busy || view.pending || !frame?.can_act || !offer?.enabled;
    this.request.title = offer?.blocked_reason?.replaceAll("_", " ") ?? "";
  }
}
