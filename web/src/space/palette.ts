import { Color } from "three";
import type { Preset } from "../presets";

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
      ambientIntensity: 0.82,
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
        ambientIntensity: 1.2,
        key: new Color("#c5d9ff"),
        keyIntensity: 1.5,
        lanternIntensity: 65,
        candleIntensity: 7,
        practicalShaderStrength: 5,
      }
    : {
        background: new Color("#091426"),
        ambient: new Color("#f2f7ff"),
        ambientIntensity: 1.2,
        key: new Color("#a9caff"),
        keyIntensity: 1,
        lanternIntensity: 55,
        candleIntensity: 5,
        practicalShaderStrength: 4,
      };
}
