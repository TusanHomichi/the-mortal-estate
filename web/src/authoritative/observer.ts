import { WireCodec } from "./codec";
import { AuthoritativeState } from "./state";
import { AuthoritativeRenderer } from "./renderer";

/** Ticket-only diagnostic observer. Authentication stays with the local harness;
 * neither cookies nor passwords enter the page or the renderer. */
export class BrowserObserver {
  state: AuthoritativeState;
  private socket: WebSocket | null = null;
  private recording: string[] = [];
  error: string | null = null;

  constructor(readonly codec: WireCodec, readonly view: AuthoritativeRenderer) {
    this.state = new AuthoritativeState(codec);
  }

  private accept(raw: string): void {
    if (!this.state.accept(raw)) return;
    if (this.recording.length >= 256) throw new Error("diagnostic recording limit reached");
    this.recording.push(JSON.stringify(this.state.snapshot!.envelope));
    this.view.present(this.state.snapshot!);
  }

  async connect(endpoint: string, ticket: string): Promise<void> {
    this.disconnect(); this.state = new AuthoritativeState(this.codec); this.error = null;
    const url = new URL(endpoint);
    if (url.protocol !== "wss:" || url.origin.replace("wss:", "https:") !== location.origin) {
      throw new Error("observer requires same-origin WSS");
    }
    const hello = this.codec.decode("client_hello_envelope", JSON.stringify({ kind: "client_hello", ticket, supported_minors: [this.codec.protocolMinor] }));
    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(endpoint, "tme.v1");
      this.socket = socket;
      const timeout = window.setTimeout(() => fail("observer welcome timed out"), 30_000);
      const fail = (message: string) => {
        if (this.socket !== socket) return;
        window.clearTimeout(timeout);
        this.error = message;
        this.disconnect();
        reject(new Error(message));
      };
      socket.onopen = () => {
        if (socket.protocol !== "tme.v1") { fail("observer subprotocol refused"); return; }
        socket.send(JSON.stringify(hello));
      };
      socket.onmessage = event => {
        if (this.socket !== socket) return;
        try {
          if (typeof event.data !== "string") throw new Error("binary envelope refused");
          this.accept(event.data);
          if (this.state.snapshot) { window.clearTimeout(timeout); resolve(); }
        } catch { fail("observer wire or frame refused"); }
      };
      socket.onerror = () => fail("observer transport failed");
      socket.onclose = () => fail("observer disconnected");
    });
  }

  replay(raw: readonly string[]): void {
    this.disconnect(); this.state = new AuthoritativeState(this.codec); this.error = null;
    try {
      if (!raw.length || raw.length > 256) throw new Error("invalid recording length");
      for (const message of raw) this.accept(message);
    } catch (error) { this.disconnect(); throw error; }
  }

  recorded(): readonly string[] {
    if (!this.state.snapshot || this.error) throw new Error(this.error ?? "no authoritative frame");
    return [...this.recording];
  }

  disconnect(): void {
    const socket = this.socket; this.socket = null;
    if (socket) { socket.onclose = null; socket.onmessage = null; socket.onerror = null; socket.close(); }
    this.state.reset(); this.recording = []; this.view.clear();
  }
}
