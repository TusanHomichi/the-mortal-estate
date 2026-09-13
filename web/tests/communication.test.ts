import { readFileSync } from "node:fs";
import path from "node:path";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { WireCodec } from "../src/authoritative/codec";
import { Communication } from "../src/play/communication";
import { MESSAGE_BACKLOG, PlayControl, type ControlView, type Transport } from "../src/play/control";

// Ordinary speech travels the existing social_message envelope: fixture-verified shape,
// no gameplay command sequence, and no can_act gate.
const root = path.resolve(import.meta.dirname, "../..");
// Distinct canonical lowercase hyphenated UUIDs: the codec refuses any other message-id text.
const wireId = (index: number) => `018f0f9f-9b5a-7c61-8d2d-${String(index).padStart(12, "0")}`;
const fixture = (file: string, id?: string) => {
  const rows = JSON.parse(readFileSync(path.join(root, `tests/fixtures/wire/${file}.json`), "utf8")).cases;
  return JSON.parse(rows.find((row: { case_id: string; expect: string }) => id ? row.case_id === id : row.expect === "accept").input_utf8);
};
let codec: WireCodec;
beforeAll(async () => { codec = await WireCodec.create(readFileSync(path.join(root, "target/wasm32-unknown-unknown/release/tme_protocol.wasm"))); });
const active: PlayControl[] = [];
afterEach(() => { for (const control of active.splice(0)) control.dispose(); });
class Socket {
  protocol = "tme.v1"; onopen: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null; onerror: (() => void) | null = null;
  sent: string[] = []; closed = false;
  constructor(readonly welcome: unknown) { queueMicrotask(() => this.onopen?.()); }
  send(bytes: string): void { this.sent.push(bytes); if (JSON.parse(bytes).kind === "client_hello") queueMicrotask(() => this.receive(this.welcome)); }
  receive(value: unknown): void { this.onmessage?.({ data: JSON.stringify(value) }); }
  close(): void { this.closed = true; }
}
async function connected(welcome?: (value: Record<string, unknown>) => void) {
  const sockets: Socket[] = [], requests: { path: string; options: RequestInit }[] = [];
  let epoch = 11;
  let messageNumber = 0;
  const login = fixture("login_response_v1");
  const transport: Transport = {
    uuid: () => wireId(messageNumber++),
    socket: () => {
      const envelope = { ...fixture("server_envelope", "accept_server_welcome"), control_epoch: String(epoch++) };
      welcome?.(envelope);
      const socket = new Socket(envelope); sockets.push(socket); return socket as unknown as WebSocket;
    },
    fetch: (async (url, options) => {
      const route = String(url).split("/v4")[1]!;
      requests.push({ path: route, options: options! });
      const body = route === "/login" ? login : route === "/session" ? { ...login.bootstrap, selected_character_id: login.bootstrap.characters[0].character_id }
        : route === "/characters/select" ? fixture("character_selection_v1") : fixture("socket_ticket_v1");
      return route === "/logout" ? new Response(null, { status: 204 }) : new Response(JSON.stringify(body));
    }) as typeof fetch,
  };
  const control = new PlayControl(codec, "https://localhost:18743", () => {}, transport); active.push(control);
  await control.login("tester", "a sufficiently long password");
  await control.select(control.view.characters[0]!.character_id);
  const result = (id: string, disposition: unknown) => ({ ...fixture("server_envelope", "accept_command_result"), command_id: id, disposition });
  return { control, sockets, result };
}
const spoken = (control: PlayControl) => control.view.messages.map(message => message.text);

describe("ordinary local speech over the social envelope", () => {
  it("shows sent speech only after its matching acknowledgement and reports server refusals", async () => {
    const {control,sockets} = await connected();
    expect(control.sendSay("Can anyone hear me?")).toBe(true);
    expect(control.view.messages).toHaveLength(0);
    const id = JSON.parse(sockets[0]!.sent.at(-1)!).message_id;
    sockets[0]!.receive({kind:"message_result",message_id:wireId(999),disposition:"accepted"});
    expect(control.view.messages).toHaveLength(0);
    sockets[0]!.receive({kind:"message_result",message_id:id,disposition:"accepted"});
    expect(spoken(control)).toEqual(["Can anyone hear me?"]);
    expect(control.view.phase).toBe("playing");expect(control.view.nextSequence).toBe("1");
    sockets[0]!.receive({kind:"message_result",message_id:id,disposition:"accepted"});
    expect(control.view.messages).toHaveLength(1);
    expect(control.sendSay("too quickly")).toBe(true);
    sockets[0]!.receive({kind:"message_result",message_id:JSON.parse(sockets[0]!.sent.at(-1)!).message_id,disposition:"rate_limited"});
    expect(control.view.feedback).toContain("too quickly");expect(control.view.messages).toHaveLength(1);
  });
  it("sends exactly the fixture-shaped social_message with live identity fields and no command cursor", async () => {
    const { control, sockets } = await connected();
    const sequence = control.view.nextSequence;
    expect(control.sendSay("Synthetic fixture message")).toBe(true);
    const sent = JSON.parse(sockets[0]!.sent.at(-1)!);
    const expected = { ...fixture("client_command_envelope", "accept_social_say"), message_id: wireId(0),
      control_epoch: control.view.snapshot!.envelope.control_epoch,
      actor_id: control.view.snapshot!.envelope.frame.observer_actor_id };
    expect(sent).toEqual(expected);
    // The accepted fixture carries no gameplay cursor or world revision.
    expect(sent.client_sequence).toBeUndefined();
    expect(sent.observed_world_revision).toBeUndefined();
    expect(sent.message_id).toBe(wireId(0));
    // The codec's accepted output is what actually goes on the wire.
    expect(codec.decode("client_command_envelope", sockets[0]!.sent.at(-1)!)).toEqual(expected);
    expect(control.sendSay("second")).toBe(true);
    expect(JSON.parse(sockets[0]!.sent.at(-1)!).message_id).toBe(wireId(1));
    expect(control.view.nextSequence).toBe(sequence);
    expect(control.view.pending).toBe(false);
  });

  it("speaks while can_act is false, where gameplay commands are refused", async () => {
    const { control, sockets } = await connected(envelope => {
      (envelope.frame as { can_act: boolean }).can_act = false;
    });
    expect(control.view.snapshot!.envelope.frame.can_act).toBe(false);
    expect(control.command({ kind: "wait" })).toBe(false);
    const sentBefore = sockets[0]!.sent.length;
    expect(control.sendSay("dead men tell tales")).toBe(true);
    expect(sockets[0]!.sent).toHaveLength(sentBefore + 1);
    expect(JSON.parse(sockets[0]!.sent.at(-1)!).body).toBe("dead men tell tales");
    expect(control.view.nextSequence).toBe("1"); expect(control.view.pending).toBe(false);
  });

  it("refuses empty, oversized, control-bearing and too-long-in-bytes bodies without sending", async () => {
    const { control, sockets } = await connected();
    const sentBefore = sockets[0]!.sent.length;
    for (const body of ["", "a".repeat(281), "\u0007", `ok${"\u0000"}`, "\uD800", "𝄞".repeat(257)]) {
      expect(control.sendSay(body)).toBe(false);
    }
    expect(sockets[0]!.sent).toHaveLength(sentBefore);
    // The same bounds accept the longest allowed scalar and byte shapes.
    expect(control.sendSay("a".repeat(280))).toBe(true);
    expect(control.sendSay("𝄞".repeat(256))).toBe(true);
    expect(control.sendSay("e\u0301")).toBe(true);
  });

  it("delivers decoded incoming speech without touching authority, and keeps only the newest lines", async () => {
    const { control, sockets } = await connected();
    const snapshot = control.view.snapshot, sequence = control.view.nextSequence;
    const incoming = fixture("server_envelope", "accept_social_message_say");
    sockets[0]!.receive(incoming);
    expect(control.view.messages).toEqual([{ messageId: incoming.message_id, scope: "say",
      senderCharacterId: incoming.sender_character_id, senderName: incoming.sender_name, text: incoming.body }]);
    expect(control.view.snapshot).toBe(snapshot); expect(control.view.nextSequence).toBe(sequence);
    expect(control.view.phase).toBe("playing");
    // A shout keeps its scope label; unrelated envelopes are not speech.
    sockets[0]!.receive(fixture("server_envelope", "accept_social_message_shout"));
    expect(control.view.messages.at(-1)!.scope).toBe("shout");
    expect(control.sendSay("")).toBe(false); expect(control.view.messages).toHaveLength(2);
    for (let index = 0; index < MESSAGE_BACKLOG + 5; ++index) {
      sockets[0]!.receive({ ...incoming, message_id: wireId(index + 1), body: `line ${index}` });
    }
    expect(control.view.messages).toHaveLength(MESSAGE_BACKLOG);
    expect(spoken(control).at(-1)).toBe(`line ${MESSAGE_BACKLOG + 4}`);
    expect(spoken(control)).not.toContain("line 0");
    expect(spoken(control).at(0)).toBe(`line ${MESSAGE_BACKLOG + 5 - MESSAGE_BACKLOG}`);
  });

  it("drops the whole transcript on authority loss and refuses speech while detached", async () => {
    const { control, sockets } = await connected();
    control.sendSay("hello");
    sockets[0]!.receive(fixture("server_envelope", "accept_social_message_say"));
    expect(control.view.messages).toHaveLength(1);
    sockets[0]!.onclose?.();
    expect(control.view.messages).toEqual([]);
    expect(control.view.phase).toBe("disconnected");
    expect(control.sendSay("still there?")).toBe(false);
    await control.reconnect();
    control.sendSay("back");
    sockets[1]!.receive(fixture("server_envelope", "accept_social_message_say"));
    expect(control.view.messages).toHaveLength(1);
    control.dispose();
    expect(control.view.messages).toEqual([]);
  });

  it("presents delivered speech and permits speaking while can_act is false", async () => {
    vi.stubGlobal("document", { createElement: () => new FakeElement() });
    const { control, sockets } = await connected(envelope => {
      (envelope.frame as { can_act: boolean }).can_act = false;
    });
    const mount = new FakeElement();
    const panel = new Communication(mount as unknown as HTMLElement, text => control.sendSay(text));
    panel.present(control.view);
    expect(control.sendSay("through the panel")).toBe(true);
    sockets[0]!.receive(fixture("server_envelope", "accept_social_message_say"));
    panel.present(control.view);
    const [form, transcript] = mount.children as [FakeElement, FakeElement];
    expect((transcript.children[0] as FakeElement).textContent).toBe("Wayfarer: Synthetic fixture message");
    expect((form.children[0] as FakeElement).disabled).toBe(false);
  });
});

class FakeElement {
  textContent = ""; value = ""; disabled = false; scrollTop = 0; scrollHeight = 0;
  name = ""; type = "";
  readonly attributes = new Map<string, string>();
  private readonly listeners: { type: string; handler: (event: { preventDefault(): void }) => void }[] = [];
  readonly children: FakeElement[] = [];
  setAttribute(name: string, value: string): void { this.attributes.set(name, value); }
  append(...nodes: FakeElement[]): void { this.children.push(...nodes); }
  replaceChildren(...nodes: FakeElement[]): void { this.children.length = 0; this.children.push(...nodes); }
  addEventListener(type: string, handler: (event: { preventDefault(): void }) => void): void { this.listeners.push({ type, handler }); }
  fire(type: string): void {
    let prevented = false;
    for (const listener of this.listeners) if (listener.type === type) listener.handler({ preventDefault: () => { prevented = true; } });
    if (!prevented && type === "submit") throw new Error("submit must be prevented");
  }
}
function panel() {
  const root = new FakeElement();
  let accepted = true;
  const send = vi.fn((_text: string) => accepted);
  const view = (overrides: Partial<ControlView> = {}): ControlView => ({
    phase: "playing", busy: false, characters: [], snapshot: null, pending: false, feedback: "",
    nextSequence: "1", creationOptions: [], createdCharacterId: null, creationRetry: null, messages: [], ...overrides,
  });
  return { root, send, view, panel: new Communication(root as unknown as HTMLElement, send) };
}

describe("local speech panel", () => {
  beforeEach(() => { vi.stubGlobal("document", { createElement: () => new FakeElement() }); });
  afterEach(() => { vi.unstubAllGlobals(); });

  it("submits one local-speech form and clears the field only when speech left the client", () => {
    const p = panel();
    const form = p.root.children[0]!, input = form.children[0] as FakeElement, button = form.children[1] as FakeElement;
    expect(p.root.children).toHaveLength(2);
    expect(form.attributes.get("aria-label")).toBe("Local speech");
    expect(input.name).toBe("say"); expect(button.type).toBe("submit");
    input.value = "  spaced out  ";
    form.fire("submit");
    expect(p.send).toHaveBeenCalledExactlyOnceWith("  spaced out  ");
    expect(input.value).toBe("");
    p.send.mockReturnValue(false);
    input.value = "refused words"; form.fire("submit");
    expect(input.value).toBe("refused words");
  });

  it("stays open for a disabled, waiting or dead character and closes only outside play", () => {
    const p = panel();
    const form = p.root.children[0]!, input = form.children[0] as FakeElement, button = form.children[1] as FakeElement;
    for (const view of [p.view({ busy: true }), p.view({ pending: true }), p.view()]) {
      p.panel.present(view);
      expect(input.disabled).toBe(false); expect(button.disabled).toBe(false);
    }
    for (const phase of ["signed_out", "selecting", "connecting", "disconnected"] as const) {
      p.panel.present(p.view({ phase }));
      expect(input.disabled).toBe(true); expect(button.disabled).toBe(true);
    }
  });

  it("renders a bounded accessible transcript as plain text and refreshes on identity", () => {
    const p = panel();
    const transcript = p.root.children[1] as FakeElement;
    const view = (overrides: Partial<ControlView> = {}) => p.view(overrides);
    expect(transcript.attributes.get("role")).toBe("log");
    expect(transcript.attributes.get("aria-live")).toBe("polite");
    expect(transcript.attributes.get("aria-label")).toBe("Local speech transcript");
    const message = (id: string, scope: string, text: string) =>
      ({ messageId: id, scope, senderCharacterId: "c", senderName: "Wayfarer", text });
    const first = [message("1", "say", "plain words"), message("2", "shout", "<b>not markup</b>")];
    p.panel.present(view({ messages: first }));
    expect(transcript.children).toHaveLength(2);
    expect(transcript.children[0]!.textContent).toBe("Wayfarer: plain words");
    expect(transcript.children[1]!.textContent).toBe("Wayfarer (shout): <b>not markup</b>");
    expect(transcript.children[0]!.children).toHaveLength(0);
    const retained = transcript.children[0];
    p.panel.present(view({ messages: first }));
    expect(transcript.children[0]).toBe(retained);
    p.panel.present(view({ messages: [...first, message("3", "say", "later")] }));
    expect(transcript.children.at(-1)!.textContent).toBe("Wayfarer: later");
    const backlog = Array.from({ length: MESSAGE_BACKLOG + 5 }, (_, index) => message(String(index), "say", `line ${index}`));
    p.panel.present(view({ messages: backlog }));
    expect(transcript.children).toHaveLength(backlog.length);
    p.panel.present(view({ phase: "disconnected" }));
    expect(transcript.children).toHaveLength(0);
  });
});
