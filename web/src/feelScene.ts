import { decodeStructures } from "./space/structures";
import type { FigureFacing } from "./walk/facing";
import {
  Clock,
  DataTexture,
  Mesh,
  OrthographicCamera,
  PlaneGeometry,
  Scene,
  ShaderMaterial,
  Vector4,
  WebGLRenderer,
  NoToneMapping,
  SRGBColorSpace,
} from "three";
import {
  cameraFocusFor,
  cameraFollowsCaretaker,
  createFeelCamera,
  focusFeelCamera,
  resizeFeelCamera,
} from "./camera";
import type { PortalTarget, VerifiedAssetPacket } from "./feelTypes";
import { describeView, type Preset } from "./presets";
import { fogFragmentShader, fogVertexShader } from "./shaders";
import { SpaceScene } from "./space/SpaceScene";
import { decodeTextures } from "./space/textures";
import { decodeFigures, disposeDecodedFigures, disposeFigureSources, type DecodedFigure } from "./space/figureRig";
import type { Cell } from "./walk/layoutPassability";
import { createWalkPresenter, type WalkPresenter } from "./walk/walkPresenter";

interface FogOverlay {
  scene: Scene;
  camera: OrthographicCamera;
  material: ShaderMaterial;
  dispose(): void;
}

interface FeelDevHook {
  wallRunPlasterOpacity(runIndex: number): number | null;
}

declare global {
  interface Window {
    __tmeFeel?: FeelDevHook;
  }
}

export interface FeelViewOptions {
  /** Comparison zoom steps from the ruled frame; 0 is the ruled frame. */
  zoomStep: number;
}

export interface FeelSceneHandle {
  renderer: WebGLRenderer;
  camera: OrthographicCamera;
  stop: () => void;
}

function makeFogOverlay(): FogOverlay {
  const scene = new Scene();
  const camera = new OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const geometry = new PlaneGeometry(2, 2);
  const material = new ShaderMaterial({
    uniforms: {
      elapsed: { value: 0 },
      fogColour: { value: new Vector4(0.33, 0.39, 0.52, 0.16) },
    },
    vertexShader: fogVertexShader,
    fragmentShader: fogFragmentShader,
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  scene.add(new Mesh(geometry, material));
  return {
    scene,
    camera,
    material,
    dispose: () => {
      geometry.dispose();
      material.dispose();
      scene.clear();
    },
  };
}

export async function startFeelScene(
  stage: HTMLElement,
  packet: VerifiedAssetPacket,
  presets: readonly Preset[],
  view: FeelViewOptions = { zoomStep: 0 },
): Promise<FeelSceneHandle> {
  const canvas = document.createElement("canvas");
  canvas.setAttribute("aria-hidden", "true");
  const context = canvas.getContext("webgl2", {
    alpha: false,
    antialias: true,
    depth: true,
    stencil: false,
  });
  if (context === null) throw new Error("WebGL2 is unavailable in this browser");
  stage.prepend(canvas);

  const renderer = new WebGLRenderer({ canvas, context, antialias: true });
  renderer.outputColorSpace = SRGBColorSpace;
  renderer.toneMapping = NoToneMapping;
  renderer.shadowMap.enabled = true;
  renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
  renderer.setSize(window.innerWidth, window.innerHeight, false);

  const textures = await decodeTextures(packet);
  let figures: Map<string, DecodedFigure>;
  try {
    figures = await decodeFigures(packet);
  } catch (error) {
    // A refused figure refuses the scene; nothing created so far may outlive it.
    for (const decoded of textures.values()) decoded.texture.dispose();
    renderer.dispose();
    canvas.remove();
    throw error;
  }
  let structures;
  try {
    structures = await decodeStructures(packet);
  } catch (error) {
    disposeDecodedFigures(figures);
    for (const decoded of textures.values()) decoded.texture.dispose();
    renderer.dispose();
    canvas.remove();
    throw error;
  }
  const windWeightTextures = new Map<string, DataTexture>();
  const anisotropy = renderer.capabilities.getMaxAnisotropy();
  const scene = new Scene();
  const initialCell = { i: packet.manifest.start.cell[0], j: packet.manifest.start.cell[1] };
  const camera = createFeelCamera(
    window.innerWidth,
    window.innerHeight,
    initialCell,
    view.zoomStep,
  );
  const clock = new Clock();
  let activeSpace: SpaceScene | null = null;
  let activePresenter: WalkPresenter | null = null;
  let activeFog: FogOverlay | null = null;
  let animationFrame = 0;
  let stopped = false;

  const swapSpace = (target: PortalTarget, facing: FigureFacing): void => {
    const space = packet.manifest.spaces[target.space];
    if (space === undefined) {
      throw new Error(`portal landing names absent verified space ${target.space}`);
    }
    const targetCell: Cell = { i: target.cell[0], j: target.cell[1] };
    activePresenter?.stop();
    activeSpace?.dispose();
    activeFog?.dispose();
    activeFog = null;

    const focus = cameraFocusFor(space, targetCell);
    focusFeelCamera(camera, focus);
    stage.dataset.cameraFocus = `${focus.i},${focus.j}`;
    const nextSpace = new SpaceScene({
      name: target.space,
      space,
      textures,
      windWeightTextures,
      presets,
      anisotropy,
      camera,
      caretakerCell: targetCell,
      caretakerFacing: facing,
      figures,
      structures,
      caretakerFigure: packet.manifest.caretaker.figure,
    });
    activeSpace = nextSpace;
    stage.dataset.caretakerFigure = nextSpace.caretaker.name;
    stage.dataset.caretakerClip = nextSpace.caretaker.clip;
    stage.dataset.grassInstances = String(nextSpace.grassInstanceCount);
    scene.background = nextSpace.background;
    scene.add(nextSpace.group);
    const presetLabel = document.querySelector<HTMLElement>("#preset-label");
    if (presetLabel !== null) {
      presetLabel.textContent = describeView(
        nextSpace.weatherEnabled ? presets.join(" · ") : "INTERIOR",
        view.zoomStep,
      );
    }
    if (nextSpace.weatherEnabled && presets.includes("fog")) {
      activeFog = makeFogOverlay();
    }
    activePresenter = createWalkPresenter({
      stage,
      canvas,
      scene,
      camera,
      spaceName: target.space,
      space,
      caretaker: nextSpace.caretaker,
      initialCell: targetCell,
      updateWallFade: (cell, now) => nextSpace.updateWallFade(cell, now),
      onCellChanged: (previous, next) => nextSpace.focusLighting(previous, next),
      onPortalLanding: swapSpace,
      cameraFollowsCaretaker: cameraFollowsCaretaker(space),
    });
  };

  swapSpace(packet.manifest.start, { i: 1, j: 0 });

  const devHook: FeelDevHook | null = import.meta.env["DEV"]
    ? {
        wallRunPlasterOpacity: (runIndex) =>
          activeSpace?.wallRunPlasterOpacity(runIndex) ?? null,
      }
    : null;
  if (devHook !== null) window.__tmeFeel = devHook;

  const draw = (): void => {
    if (stopped) return;
    const elapsed = clock.getElapsedTime();
    activeSpace?.update(elapsed);
    activePresenter?.update(performance.now() / 1000);
    renderer.setRenderTarget(null);
    renderer.clear();
    const renderStartedAt = performance.now();
    renderer.render(scene, camera);
    if (activeFog !== null) {
      activeFog.material.uniforms.elapsed!.value = elapsed;
      renderer.autoClear = false;
      renderer.render(activeFog.scene, activeFog.camera);
      renderer.autoClear = true;
    }
    stage.dataset.renderCalls = String(renderer.info.render.calls);
    stage.dataset.renderMilliseconds = (performance.now() - renderStartedAt).toFixed(3);
    animationFrame = requestAnimationFrame(draw);
  };

  const resize = (): void => {
    renderer.setSize(window.innerWidth, window.innerHeight, false);
    resizeFeelCamera(camera, window.innerWidth, window.innerHeight, view.zoomStep);
  };
  window.addEventListener("resize", resize);
  draw();
  return {
    renderer,
    camera,
    stop: () => {
      stopped = true;
      cancelAnimationFrame(animationFrame);
      window.removeEventListener("resize", resize);
      activePresenter?.stop();
      activeSpace?.dispose();
      activeFog?.dispose();
      for (const decoded of textures.values()) decoded.texture.dispose();
      for (const texture of windWeightTextures.values()) texture.dispose();
      disposeDecodedFigures(figures);
      disposeFigureSources([...structures.values()]);
      delete stage.dataset.caretakerFigure;
      delete stage.dataset.caretakerClip;
      if (devHook !== null && window.__tmeFeel === devHook) delete window.__tmeFeel;
      delete stage.dataset.renderCalls;
      delete stage.dataset.renderMilliseconds;
      delete stage.dataset.grassInstances;
      renderer.dispose();
    },
  };
}
