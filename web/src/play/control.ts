import { feedbackText, type FeedbackEvent } from "./feedback";
import { WireCodec } from "../authoritative/codec";
import { AuthoritativeState, type Snapshot } from "../authoritative/state";
import { offeredIntent, withGoldAmount } from "../authoritative/gameplay";
import type { PathPreview, PathPreviewResult } from "./pathPreview";

export type Phase = "signed_out" | "selecting" | "connecting" | "playing" | "disconnected";
export interface Character { character_id: string; slot: number; display_name: string }
export interface Attributes { strength: number; dexterity: number; constitution: number; intelligence: number; wisdom: number; charisma: number }
export interface CreationOption { profile_id: string; class_name: string; nationality: string; minimum: Attributes; maximum: Attributes; suggested: Attributes; attribute_points: number }
export interface CreationDraft { profile_id: string; display_name: string; attributes: Attributes }
interface Bootstrap { csrf_token: string; characters: Character[]; selected_character_id: string | null }
interface Login { session_token: string; bootstrap: Bootstrap }
interface Ticket { ticket: string }
interface Result { kind: "command_result"; command_id: string; disposition: { kind: string; code?: string }; events: FeedbackEvent[] }
interface Pending { readonly id: string; readonly epoch: string; readonly sequence: bigint; readonly bytes: string }
export interface ControlView {
  phase: Phase; busy: boolean; characters: readonly Character[]; snapshot: Snapshot | null;
  pending: boolean; feedback: string; nextSequence: string;
  creationOptions: readonly CreationOption[]; createdCharacterId: string | null;
}
export interface Transport {
  fetch: typeof fetch;
  socket(url: string): WebSocket;
  uuid(): string;
}
const native: Transport = { fetch: (...args) => fetch(...args), socket: url => new WebSocket(url, "tme.v1"), uuid: () => crypto.randomUUID() };

/** The only owner of credentials, HTTP/WSS, the epoch cursor and pending bytes.
 * Its public view contains presentation facts, never authentication secrets. */
export class PlayControl {
  private readonly state: AuthoritativeState;
  private token: string | null = null;
  private bootstrap: Bootstrap | null = null;
  private socket: WebSocket | null = null;
  private phase: Phase = "signed_out";
  private epoch: string | null = null;
  private nextSequence = 1n;
  private pending: Pending | null = null;
  private preview: { id: string; finish: (value: PathPreviewResult | null) => void } | null = null;
  private feedback = "Sign in to enter the private world.";
  private queue: Promise<unknown> = Promise.resolve();
  private busy = 0;
  private creationOptions: CreationOption[] = [];
  private createdCharacterId: string | null = null;
  private pendingCreation: { requestId: string; draft: CreationDraft; fingerprint: string } | null = null;
  private commandTimer: ReturnType<typeof setTimeout> | undefined;
  private reconnectTimer: ReturnType<typeof setTimeout> | undefined;
  private readonly root: string;

  constructor(readonly codec: WireCodec, origin: string,
    private readonly changed: (view: ControlView) => void, private readonly transport: Transport = native) {
    const url = new URL(origin);
    if (url.protocol !== "https:" || url.origin !== origin) throw new Error("Canonical HTTPS origin required");
    this.root = `${origin}/v${codec.controlVersion}`;
    this.state = new AuthoritativeState(codec);
    this.emit();
  }

  get view(): ControlView {
    return { phase: this.phase, busy: this.busy > 0, characters: this.bootstrap?.characters ?? [],
      snapshot: this.state.snapshot, pending: this.pending !== null, feedback: this.feedback, nextSequence: this.nextSequence.toString(),
      creationOptions: this.creationOptions, createdCharacterId: this.createdCharacterId };
  }
  private emit(): void { this.changed(this.view); }
  private serial<T>(work: () => Promise<T>): Promise<T> {
    ++this.busy; this.emit();
    const result = this.queue.then(work).catch(error => {
      // Transport errors may contain URLs; rejected documents may contain secrets.
      if (this.token && this.phase === "connecting") { this.detach(); this.phase = "disconnected"; }
      this.feedback = error instanceof ControlFailure ? error.message : "Connection failed. Reconnect to recover control state.";
      throw new Error(this.feedback);
    }).finally(() => { --this.busy; this.emit(); });
    this.queue = result.catch(() => {});
    return result;
  }

  private async request<T>(path: string, decoder: string | null, request?: { decoder: string; value: unknown }): Promise<T> {
    const body = request ? JSON.stringify(this.codec.decode(request.decoder, JSON.stringify(request.value))) : undefined;
    const headers: Record<string, string> = { Accept: "application/json" };
    if (body !== undefined) headers["Content-Type"] = "application/json";
    if (this.token) headers.Authorization = `Bearer ${this.token}`;
    const response = await this.transport.fetch(this.root + path, {
      method: body === undefined ? "GET" : "POST", headers, body, credentials: "omit",
      cache: "no-store", redirect: "error", signal: AbortSignal.timeout(15_000),
    });
    if (response.status === 204 && !decoder) return undefined as T;
    const reader = response.body?.getReader();
    if (!reader) throw new Error("Missing response");
    const chunks: Uint8Array[] = []; let length = 0;
    try {
      for (;;) {
        const { value, done } = await reader.read(); if (done) break;
        length += value.byteLength;
        if (length > this.codec.controlLimit) throw new Error("Response too large");
        chunks.push(value);
      }
    } finally { await reader.cancel(); }
    const bytes = new Uint8Array(length); let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
    if (!response.ok) {
      const error = this.codec.decode<{ code: string }>("control_error_v1", bytes);
      if (response.status === 401 && this.token) this.clear();
      throw new ControlFailure(error.code === "invalid_credentials" ? "Sign-in details were refused." : `Request refused: ${error.code.replaceAll("_", " ")}.`, response.status < 500);
    }
    if (!decoder) throw new Error("Unexpected response body");
    return this.codec.decode<T>(decoder, bytes);
  }

  login(username: string, password: string): Promise<void> {
    return this.serial(async () => {
      if (this.token) throw new ControlFailure("Sign out before changing accounts.");
      const value = await this.request<Login>("/login", "login_response_v1", { decoder: "login_request_v1", value: { username, password } });
      password = "";
      this.token = value.session_token; this.bootstrap = value.bootstrap;
      this.phase = "selecting"; this.feedback = "Choose a character.";
    });
  }
  select(characterId: string): Promise<void> {
    return this.serial(async () => {
      if (!this.bootstrap || this.phase !== "selecting") throw new ControlFailure("Sign in first.");
      await this.request("/characters/select", "character_selection_v1", { decoder: "character_select_request_v1", value: { csrf_token: this.bootstrap.csrf_token, character_id: characterId } });
      await this.connect();
    });
  }
  openCreation(): Promise<void> {
    return this.serial(async () => {
      if (!this.bootstrap || this.phase !== "selecting") throw new ControlFailure("Sign in first.");
      const value = await this.request<{ options: CreationOption[] }>("/characters/creation", "character_creation_options_v1",
        { decoder: "session_bootstrap_request_v1", value: {} });
      this.creationOptions = value.options;
      this.feedback = value.options.length ? "Choose a class and allocate your attribute points." : "This world has no character creation profiles.";
    });
  }
  createCharacter(draft: CreationDraft): Promise<void> {
    return this.serial(async () => {
      if (!this.bootstrap || this.phase !== "selecting") throw new ControlFailure("Sign in first.");
      const fingerprint = JSON.stringify(draft);
      if (this.pendingCreation && this.pendingCreation.fingerprint !== fingerprint) {
        throw new ControlFailure("Retry the previous creation unchanged to resolve its outcome.");
      }
      const pending = this.pendingCreation ?? { requestId: this.transport.uuid(), draft: structuredClone(draft), fingerprint };
      this.codec.decode("character_create_request_v1", JSON.stringify({ csrf_token: this.bootstrap.csrf_token, request_id: pending.requestId, draft: pending.draft }));
      this.pendingCreation = pending;
      let value: { character: Character };
      try {
        value = await this.request("/characters/create", "character_created_v1", {
          decoder: "character_create_request_v1", value: { csrf_token: this.bootstrap.csrf_token,
            request_id: pending.requestId, draft: pending.draft },
        });
      } catch (error) {
        if (error instanceof ControlFailure && error.definitive) this.pendingCreation = null;
        throw error;
      }
      this.pendingCreation = null;
      this.bootstrap.characters = [...this.bootstrap.characters.filter(row => row.character_id !== value.character.character_id), value.character].sort((a, b) => a.slot - b.slot);
      this.createdCharacterId = value.character.character_id;
      this.creationOptions = [];
      this.feedback = "Character created. Enter the world when you are ready.";
    });
  }
  reconnect(): Promise<void> {
    return this.serial(async () => {
      if (!this.token) throw new ControlFailure("Sign in again.");
      this.detach(); this.phase = "connecting"; this.emit();
      this.bootstrap = await this.request<Bootstrap>("/session", "session_bootstrap_v1", { decoder: "session_bootstrap_request_v1", value: {} });
      if (!this.bootstrap.selected_character_id) { this.phase = "selecting"; this.pending = null; return; }
      await this.connect();
    });
  }
  logout(): Promise<void> {
    return this.serial(async () => {
      this.detach();
      if (!this.token) { this.clear(); return; }
      // Fresh bootstrap resolves a stale CSRF token after an ambiguous operation.
      this.phase = "disconnected";
      this.bootstrap = await this.request<Bootstrap>("/session", "session_bootstrap_v1", { decoder: "session_bootstrap_request_v1", value: {} });
      await this.request("/logout", null, { decoder: "logout_request_v1", value: { csrf_token: this.bootstrap.csrf_token } });
      this.clear(); this.feedback = "Signed out.";
    });
  }

  private async connect(): Promise<void> {
    this.detach(); this.phase = "connecting"; this.emit();
    const ticket = await this.request<Ticket>("/socket-tickets", "socket_ticket_v1", { decoder: "socket_ticket_request_v1", value: { csrf_token: this.bootstrap!.csrf_token } });
    const hello = JSON.stringify(this.codec.decode("client_hello_envelope", JSON.stringify({ kind: "client_hello", ticket: ticket.ticket, supported_minors: [this.codec.protocolMinor] })));
    await new Promise<void>((resolve, reject) => {
      const socket = this.transport.socket(this.root.replace("https:", "wss:") + "/socket");
      this.socket = socket;
      let welcomed = false;
      const timer = setTimeout(() => fail("Connection welcome timed out."), 15_000);
      const fail = (message: string, retry = false) => {
        if (this.socket !== socket) return;
        clearTimeout(timer); this.detach(); this.phase = "disconnected"; this.feedback = message; this.emit();
        if (!welcomed) reject(new ControlFailure(message));
        if (retry && welcomed && this.token) this.reconnectTimer = setTimeout(() => { void this.reconnect().catch(() => {}); }, 1_000);
      };
      socket.onopen = () => {
        if (this.socket !== socket) return;
        if (socket.protocol !== "tme.v1") { fail("Socket protocol refused."); return; }
        socket.send(hello);
      };
      socket.onmessage = event => {
        if (this.socket !== socket) return;
        try {
          if (typeof event.data !== "string") throw new Error("Binary envelope");
          const value = this.codec.decode<Result | { kind: string; reason?: string }>("server_envelope", event.data);
          if (!welcomed && value.kind !== "server_welcome") throw new Error("Welcome required");
          if (value.kind === "server_draining") { fail("Server disconnected. Reconnect when it is ready."); return; }
          if (value.kind === "error") { fail("Server refused the connection."); return; }
          if (value.kind === "path_preview_result") {
            const result = value as PathPreviewResult;
            if (this.preview?.id === result.preview_id) this.preview.finish(result);
            return;
          }
          if (value.kind === "command_result") this.settle(value as Result);
          else if (this.state.accept(event.data)) {
            if (!welcomed) {
              this.epoch = this.state.snapshot!.envelope.control_epoch!; this.nextSequence = 1n;
              welcomed = true; clearTimeout(timer); this.phase = "playing"; this.feedback = "Connected.";
              if (this.pending) { socket.send(this.pending.bytes); this.armCommandTimeout(); }
              resolve();
            }
          }
          this.emit();
        } catch { fail("Server message failed validation. Reconnect to recover."); }
      };
      socket.onerror = () => fail("Connection lost. Recovering…", true);
      socket.onclose = () => fail("Connection closed. Recovering…", true);
    });
  }

  offeredAction(generation: number, groupKey: string, actionId: string, amount?: string): boolean {
    const snapshot = this.state.snapshot;
    if (!snapshot || snapshot.generation !== generation) return false;
    const intent = offeredIntent(snapshot.envelope.frame, groupKey, actionId);
    const draft = intent && withGoldAmount(intent, amount);
    return draft !== null && this.command(draft);
  }

  cancelPathPreview(): void { this.preview?.finish(null); }

  /** Non-mutating assessment, separate from the sequenced command cursor. */
  previewPath(path: readonly string[]): Promise<PathPreview | null> {
    this.cancelPathPreview();
    const snapshot = this.state.snapshot;
    if (this.phase !== "playing" || this.busy || this.pending || !snapshot?.envelope.frame.can_act || !this.socket) return Promise.resolve(null);
    const id = this.transport.uuid(), epoch = this.epoch;
    const frame = snapshot.envelope.frame;
    const bytes = JSON.stringify(this.codec.decode("client_command_envelope", JSON.stringify({ kind: "path_preview",
      preview_id: id, control_epoch: epoch, observed_world_revision: snapshot.envelope.world_revision,
      actor_id: frame.observer_actor_id, path })));
    return new Promise(resolve => {
      const finish = (result: PathPreviewResult | null) => {
        clearTimeout(timer);
        if (this.preview?.id === id) this.preview = null;
        const current = this.state.snapshot;
        const valid = result?.disposition.kind === "previewed" && result.control_epoch === epoch && this.epoch === epoch &&
          result.actor_id === frame.observer_actor_id && result.preview?.actor_id === frame.observer_actor_id &&
          BigInt(result.world_revision) >= BigInt(snapshot.envelope.world_revision) &&
          JSON.stringify(result.preview.start) === JSON.stringify(frame.observation_center) &&
          JSON.stringify(current?.envelope.frame.observation_center) === JSON.stringify(frame.observation_center) &&
          JSON.stringify(result.preview.requested_path) === JSON.stringify(path);
        resolve(valid ? result!.preview : null);
      };
      const timer = setTimeout(() => finish(null), 5_000);
      this.preview = { id, finish };
      try { this.socket!.send(bytes); } catch { finish(null); }
    });
  }

  command(intent: unknown): boolean {
    const snapshot = this.state.snapshot;
    if (this.phase !== "playing" || this.busy || this.pending || !snapshot?.envelope.frame.can_act || !this.socket) return false;
    this.cancelPathPreview();
    const id = this.transport.uuid();
    const bytes = JSON.stringify(this.codec.decode("client_command_envelope", JSON.stringify({ kind: "command", command_id: id,
      control_epoch: this.epoch, client_sequence: this.nextSequence.toString(), observed_world_revision: snapshot.envelope.world_revision,
      actor_id: snapshot.envelope.frame.observer_actor_id, intent })));
    this.pending = Object.freeze({ id, bytes, epoch: this.epoch!, sequence: this.nextSequence });
    try { this.socket.send(bytes); this.armCommandTimeout(); }
    catch { this.detach(); this.phase = "disconnected"; this.feedback = "Command outcome unknown. Reconnect to reconcile it."; }
    this.emit(); return true;
  }
  private settle(result: Result): void {
    const pending = this.pending;
    if (!pending || result.command_id !== pending.id) return;
    const disposition = result.disposition;
    if (disposition.kind === "accepted" || (disposition.kind === "rejected" && disposition.code === "rules_rejected")) {
      if (pending.epoch === this.epoch) this.nextSequence = pending.sequence + 1n;
    }
    clearTimeout(this.commandTimer); this.pending = null;
    this.feedback = disposition.kind === "accepted" ? feedbackText(result.events).join(" ") || "Action accepted." : disposition.kind === "command_result_expired"
      ? "Command receipt expired. Reconnect before acting." : `Action refused: ${disposition.code?.replaceAll("_", " ")}.`;
    if (disposition.kind === "command_result_expired") { this.detach(); this.phase = "disconnected"; }
  }
  private armCommandTimeout(): void {
    clearTimeout(this.commandTimer);
    this.commandTimer = setTimeout(() => {
      this.detach(); this.phase = "disconnected"; this.feedback = "Command outcome unknown. Reconnect to reconcile it."; this.emit();
    }, 15_000);
  }
  private detach(): void {
    this.cancelPathPreview();
    clearTimeout(this.commandTimer); clearTimeout(this.reconnectTimer);
    const socket = this.socket; this.socket = null;
    if (socket) { socket.onopen = socket.onmessage = socket.onerror = socket.onclose = null; socket.close(); }
    this.state.reset(); this.epoch = null;
  }
  private clear(): void { this.detach(); this.token = null; this.bootstrap = null; this.pending = null; this.phase = "signed_out";
    this.creationOptions = []; this.createdCharacterId = null; this.pendingCreation = null; }
  dispose(): void { this.clear(); this.emit(); }
}
class ControlFailure extends Error {
  constructor(message: string, readonly definitive = true) { super(message); }
}
