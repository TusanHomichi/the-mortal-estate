import { WireCodec } from "./codec";
import { BrowserObserver } from "./observer";
import { AuthoritativeRenderer } from "./renderer";
import { capture, type SourceBinding } from "./capture";

const canvas = document.createElement("canvas");
document.body.append(canvas);
document.body.style.margin = "0";
const response = await fetch("/codec.wasm");
if (!response.ok) throw new Error("Rust codec unavailable");
const codec = await WireCodec.create(await response.arrayBuffer());
const view = new AuthoritativeRenderer(canvas, innerWidth, innerHeight);
const observer = new BrowserObserver(codec, view);
canvas.addEventListener("pointermove", event => {
  const rect = canvas.getBoundingClientRect();
  const target = view.pointer((event.clientX - rect.left) * view.width / rect.width,
    (event.clientY - rect.top) * view.height / rect.height);
  canvas.dataset.pointerIdentity = target?.identity ?? "";
});
const api = {
  connect: (endpoint: string, ticket: string) => observer.connect(endpoint, ticket),
  replay: (messages: string[]) => observer.replay(messages),
  capture: (route: "live" | "replay", sources: SourceBinding[]) => capture(observer, route, sources),
  pointer: (x: number, y: number) => view.pointer(x, y),
  get targets() { return view.targets; },
  get snapshot() { return view.snapshot; },
  disconnect: () => observer.disconnect(),
};
Object.assign(window, { authoritativeCapture: api });
document.body.dataset.captureReady = "true";
