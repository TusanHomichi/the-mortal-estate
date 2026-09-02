export interface DecodedRgbaPixels {
  width: number;
  height: number;
  data: Uint8Array | Uint8ClampedArray;
}

export const WIND_WEIGHT_BLUR_RADIUS = 3;

function smoothstep(value: number): number {
  const clamped = Math.min(1, Math.max(0, value));
  return clamped * clamped * (3 - 2 * clamped);
}

function isFoliage(red: number, green: number, blue: number, alpha: number): boolean {
  if (alpha < 24) return false;
  const maximum = Math.max(red, green, blue);
  const minimum = Math.min(red, green, blue);
  const saturation = maximum === 0 ? 0 : (maximum - minimum) / maximum;
  if (saturation < 0.18 || green < red + 7 || green < blue + 5) return false;

  const delta = maximum - minimum;
  const hue = delta === 0
    ? 0
    : maximum === red
      ? 60 * (((green - blue) / delta) % 6)
      : maximum === green
        ? 60 * ((blue - red) / delta + 2)
        : 60 * ((red - green) / delta + 4);
  const normalisedHue = hue < 0 ? hue + 360 : hue;
  return normalisedHue >= 55 && normalisedHue <= 175;
}

function blurMask(
  source: Float32Array,
  width: number,
  height: number,
  radius: number,
): Float32Array {
  const horizontal = new Float32Array(source.length);
  const output = new Float32Array(source.length);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let total = 0;
      let count = 0;
      for (let offset = -radius; offset <= radius; offset += 1) {
        const sampleX = x + offset;
        if (sampleX < 0 || sampleX >= width) continue;
        total += source[y * width + sampleX]!;
        count += 1;
      }
      horizontal[y * width + x] = total / count;
    }
  }
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      let total = 0;
      let count = 0;
      for (let offset = -radius; offset <= radius; offset += 1) {
        const sampleY = y + offset;
        if (sampleY < 0 || sampleY >= height) continue;
        total += horizontal[sampleY * width + x]!;
        count += 1;
      }
      output[y * width + x] = total / count;
    }
  }
  return output;
}

/**
 * Derives presentation-only wind influence from decoded card art. Image rows
 * are top-to-bottom: the card root is the final row and therefore remains
 * fixed. Foliage is selected by colour, then blurred so leaf clusters move as
 * clumps rather than as glittering individual pixels.
 */
export function buildWindWeight(
  pixels: DecodedRgbaPixels,
  kind: string,
): Uint8Array {
  const { width, height, data } = pixels;
  if (width <= 0 || height <= 0 || data.length !== width * height * 4) {
    throw new Error("wind weight source dimensions do not match its RGBA pixels");
  }

  const leafMask = new Float32Array(width * height);
  for (let index = 0; index < leafMask.length; index += 1) {
    const offset = index * 4;
    leafMask[index] = isFoliage(
      data[offset]!,
      data[offset + 1]!,
      data[offset + 2]!,
      data[offset + 3]!,
    ) ? 1 : 0;
  }
  const blurredLeaves = kind === "tree_bare"
    ? leafMask
    : blurMask(leafMask, width, height, WIND_WEIGHT_BLUR_RADIUS);
  const output = new Uint8Array(width * height);
  const heightDivisor = Math.max(1, height - 1);
  for (let y = 0; y < height; y += 1) {
    const heightTerm = smoothstep(1 - y / heightDivisor);
    for (let x = 0; x < width; x += 1) {
      const index = y * width + x;
      const alpha = data[index * 4 + 3]! / 255;
      const leafTerm = kind === "tree_bare" ? 0.15 : blurredLeaves[index]!;
      output[index] = Math.round(255 * heightTerm * leafTerm * alpha);
    }
  }
  return output;
}
