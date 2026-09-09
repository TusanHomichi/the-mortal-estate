import { Color, Vector3 } from "three";
import { lightingPeriod, type Preset } from "../presets";
import type { InteriorLighting } from "./structureLighting";

export const INTERIOR_AMBIENT_INTENSITY = 1.08;

export interface ScenePalette {
  background: Color;
  ambient: Color;
  ambientIntensity: number;
  key: Color;
  keyIntensity: number;
  lanternIntensity: number;
  candleIntensity: number;
  practicalShaderStrength: number;
}

export function paletteFor(
  presets: readonly Preset[],
  weather: boolean,
  interiorLighting: InteriorLighting | null = null,
): ScenePalette {
  if (interiorLighting) {
    if (weather) throw new Error("interior lighting profile cannot light an outdoor space");
    return {
      background: new Color("#0e0c0b"), ambient: new Color("#e8e0d3"),
      ambientIntensity: interiorLighting.indirect_intensity,
      key: new Color("#ffffff"), keyIntensity: 0,
      lanternIntensity: 0, candleIntensity: 5, practicalShaderStrength: 0,
    };
  }
  if (!weather) {
    return {
      background: new Color("#07101d"),
      ambient: new Color("#e8efff"),
      ambientIntensity: INTERIOR_AMBIENT_INTENSITY,
      key: new Color("#ffdfb2"),
      keyIntensity: 1.05,
      lanternIntensity: 0,
      candleIntensity: 5,
      practicalShaderStrength: 0,
    };
  }
  if (lightingPeriod(presets) === "day") return {
    background: new Color("#91adb8"), ambient: new Color("#d5e3ef"),
    ambientIntensity: 0.68, key: new Color("#fff0d4"), keyIntensity: 2.7,
    lanternIntensity: 0, candleIntensity: 0, practicalShaderStrength: 0,
  };
  return lightingPeriod(presets) === "dusk"
    ? {
        background: new Color("#4b394d"),
        ambient: new Color("#d2ddf0"),
        ambientIntensity: 0.9,
        key: new Color("#c5d9ff"),
        keyIntensity: 1.2,
        lanternIntensity: 14,
        candleIntensity: 7,
        practicalShaderStrength: 5,
      }
    : {
        background: new Color("#091426"),
        ambient: new Color("#9fb6d2"),
        ambientIntensity: 0.75,
        key: new Color("#a9caff"),
        keyIntensity: 0.8,
        lanternIntensity: 12,
        candleIntensity: 5,
        practicalShaderStrength: 4,
      };
}

/** Shared world-space key placement for geometry and procedural water lighting. */
export function keyLightOffset(presets: readonly Preset[], outdoors: boolean): Vector3 {
  if (!outdoors) return new Vector3(-3.5, 12, 6);
  if (lightingPeriod(presets) === "dusk") return new Vector3(-10.5, 6, 6.5);
  return lightingPeriod(presets) === "day" ? new Vector3(-9, 10, 8) : new Vector3(-7, 12, 8);
}
