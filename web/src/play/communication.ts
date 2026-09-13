import type { ControlView, SpeechMessage } from "./control";

const node = <K extends keyof HTMLElementTagNameMap>(tag: K, text: string) => {
  const element = document.createElement(tag); element.textContent = text; return element;
};

const line = (message: SpeechMessage): string => message.scope === "say"
  ? `${message.senderName}: ${message.text}`
  : `${message.senderName} (${message.scope}): ${message.text}`;

/** One plain local-speech form and its bounded transcript. The transcript is
 * presentation state: it never gates speech on can_act, and authority loss clears it. */
export class Communication {
  private readonly form = node("form", "");
  private readonly input = node("input", "");
  private readonly say = node("button", "Say");
  private readonly transcript = node("ol", "");
  private shown: readonly SpeechMessage[] | null = null;

  constructor(private readonly root: HTMLElement, private readonly send: (text: string) => boolean) {
    this.form.setAttribute("aria-label", "Local speech");
    this.input.type = "text"; this.input.name = "say"; this.input.autocomplete = "off";
    this.input.placeholder = "Say something";
    this.input.setAttribute("aria-label", "Say something to nearby characters");
    this.say.type = "submit";
    this.form.append(this.input, this.say);
    this.transcript.setAttribute("role", "log");
    this.transcript.setAttribute("aria-live", "polite");
    this.transcript.setAttribute("aria-label", "Local speech transcript");
    // Refused text stays in the field so the speaker can correct it.
    this.form.addEventListener("submit", event => {
      event.preventDefault();
      if (this.send(this.input.value)) this.input.value = "";
    });
    this.root.append(this.form, this.transcript);
  }

  present(view: ControlView): void {
    // Death, cooldown and a pending command never silence ordinary speech.
    const speaking = view.phase === "playing";
    this.input.disabled = !speaking; this.say.disabled = !speaking;
    if (view.messages === this.shown) return;
    this.shown = view.messages;
    this.transcript.replaceChildren(...view.messages.map(message => node("li", line(message))));
    this.transcript.scrollTop = this.transcript.scrollHeight;
  }
}
