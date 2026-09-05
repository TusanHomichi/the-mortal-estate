import { Color } from "three";
import type { Preset } from "../presets";

export const INTERIOR_AMBIENT_INTENSITY = 0.68;

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
): ScenePalette {
  if (!weather) {
    return {
      background: new Color("#07101d"),
      ambient: new Color("#e8efff"),
      ambientIntensity: INTERIOR_AMBIENT_INTENSITY,
      key: new Color("#9db7dc"),
      keyIntensity: 0.72,
      lanternIntensity: 0,
      candleIntensity: 5,
      practicalShaderStrength: 0,
    };
  }
  return presets.includes("dusk")
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
