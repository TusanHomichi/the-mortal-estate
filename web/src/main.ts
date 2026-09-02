import "./style.css";
import { startFeelScene } from "./feelScene";
import { fetchVerifiedAssetPacket } from "./manifest";
import { describeView, presetsFromUrl, zoomStepFromUrl } from "./presets";

const stage = document.querySelector<HTMLElement>("#feel-stage");
const banner = document.querySelector<HTMLElement>("#scene-banner");
const presetLabel = document.querySelector<HTMLElement>("#preset-label");

if (stage === null || banner === null || presetLabel === null) {
  throw new Error("the feel scene document is incomplete");
}

const pageUrl = new URL(window.location.href);
const presets = presetsFromUrl(pageUrl);
const zoomStep = zoomStepFromUrl(pageUrl);
presetLabel.textContent = describeView(presets.join(" · "), zoomStep);

function showRefusal(reason: string): void {
  stage!.dataset.sceneState = "refused";
  banner!.hidden = false;
  banner!.textContent = `FEEL SCENE — CANDIDATE ASSETS ABSENT\n${reason}`;
  document.body.dataset.sceneReady = "true";
}

async function main(): Promise<void> {
  try {
    const packet = await fetchVerifiedAssetPacket();
    await startFeelScene(stage!, packet, presets, { zoomStep });
    banner!.hidden = true;
    stage!.dataset.sceneState = "ready";
    document.body.dataset.sceneReady = "true";
  } catch (error) {
    const reason = error instanceof Error ? error.message : "candidate feel assets were refused";
    showRefusal(reason);
    console.error("Feel scene refused:", error);
  }
}

void main();
