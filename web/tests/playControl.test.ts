import { readFileSync } from "node:fs";
import path from "node:path";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { WireCodec } from "../src/authoritative/codec";
import { PlayControl, type Transport } from "../src/play/control";

const root = path.resolve(import.meta.dirname, "../..");
const fixture = (file: string, id?: string) => {
  const rows = JSON.parse(readFileSync(path.join(root, `tests/fixtures/wire/${file}.json`), "utf8")).cases;
  return JSON.parse(rows.find((row: { case_id: string; expect: string }) => id ? row.case_id === id : row.expect === "accept").input_utf8);
};
let codec: WireCodec;
beforeAll(async () => { codec = await WireCodec.create(readFileSync(path.join(root, "target/wasm32-unknown-unknown/release/tme_protocol.wasm"))); });
const active: PlayControl[] = [];
afterEach(() => { for (const control of active.splice(0)) control.dispose(); });
const turn = () => new Promise(resolve => setTimeout(resolve, 0));
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
async function connected() {
  const sockets: Socket[] = [], requests: { path: string; options: RequestInit }[] = [];
  let epoch = 11;
  const login = fixture("login_response_v1");
  const transport: Transport = {
    uuid: () => "018f0f9f-9b5a-7c61-8d2d-5ab82b1c3d4e",
    socket: () => { const socket = new Socket({ ...fixture("server_envelope", "accept_server_welcome"), control_epoch: String(epoch++) }); sockets.push(socket); return socket as unknown as WebSocket; },
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
  return { control, sockets, requests, login, result };
}
describe("actual serialized browser connection adapter", () => {
  it("uses explicit transient control auth, a strict POST bootstrap, and ticket-only sockets", async () => {
    const { control, sockets, requests, login } = await connected();
    await control.reconnect();
    for (const row of requests) {
      expect(row.options.credentials).toBe("omit");
      expect(row.options.redirect).toBe("error");
      if (row.path !== "/login") expect((row.options.headers as Record<string,string>).Authorization).toBe(`Bearer ${login.session_token}`);
    }
    expect(requests.find(row => row.path === "/session")!.options.method).toBe("POST");
    expect(JSON.parse(sockets[0]!.sent[0]!)).toEqual({ kind: "client_hello", ticket: fixture("socket_ticket_v1").ticket, supported_minors: [codec.protocolMinor] });
    expect(JSON.stringify(control.view)).not.toContain(login.session_token);
    await control.logout(); expect(control.view.phase).toBe("signed_out"); expect(control.view.snapshot).toBeNull();
  });
  it("keeps one immutable pending command and never changes authority from a result", async () => {
    const { control, sockets, result } = await connected();
    const before = control.view.snapshot;
    expect(control.command({ kind: "wait" })).toBe(true);
    expect(control.command({ kind: "wait" })).toBe(false);
    const command = JSON.parse(sockets[0]!.sent[1]!);
    expect(command.observed_world_revision).toBe(before!.envelope.world_revision);
    sockets[0]!.receive(result(command.command_id, { kind: "accepted" }));
    expect(control.view.snapshot).toBe(before); expect(control.view.nextSequence).toBe("2");
    sockets[0]!.receive(result(command.command_id, { kind: "accepted" }));
    expect(control.view.nextSequence).toBe("2");
  });
  it.each(["wrong_actor", "stale_control_epoch", "future_world_revision", "out_of_order_client_sequence", "projection_failed", "rules_rejected"])("consumes the proper cursor for %s", async code => {
    const { control, sockets, result } = await connected();
    control.command({ kind: "wait" });
    const command = JSON.parse(sockets[0]!.sent[1]!);
    sockets[0]!.receive(result(command.command_id, { kind: "rejected", code }));
    expect(control.view.nextSequence).toBe(code === "rules_rejected" ? "2" : "1");
    expect(control.view.pending).toBe(false);
  });
  it("clears authority and replays only original bytes, then settles the old epoch without consuming the new cursor", async () => {
    const { control, sockets, result } = await connected();
    control.command({ kind: "wait" });
    const original = sockets[0]!.sent[1]!;
    sockets[0]!.onclose?.();
    expect(control.view.snapshot).toBeNull(); expect(control.command({ kind: "wait" })).toBe(false);
    await control.reconnect();
    expect(sockets[1]!.sent[1]).toBe(original);
    sockets[1]!.receive(result(JSON.parse(original).command_id, { kind: "accepted" }));
    expect(control.view.pending).toBe(false); expect(control.view.nextSequence).toBe("1");
    control.command({ kind: "wait" });
    expect(JSON.parse(sockets[1]!.sent[2]!).control_epoch).toBe("12");
  });
  it("serializes simultaneous bootstrap operations instead of racing CSRF rotation", async () => {
    const { control, requests } = await connected();
    await Promise.all([control.reconnect(), control.reconnect()]);
    expect(requests.slice(-4).map(row => row.path)).toEqual(["/session", "/socket-tickets", "/session", "/socket-tickets"]);
    await turn(); expect(control.view.phase).toBe("playing");
  });
});
