import { cardWrappedDiffuseGlsl } from "./space/cardLighting";

export const groundVertexShader = /* glsl */ `
  attribute vec2 cellOrigin;
  varying vec2 vUv;
  varying vec2 vCellOrigin;
  varying vec3 vWorldPosition;
  varying vec3 vWorldNormal;

  void main() {
    vUv = uv;
    vCellOrigin = cellOrigin;
    vWorldPosition = (modelMatrix * vec4(position, 1.0)).xyz;
    vWorldNormal = normalize(mat3(modelMatrix) * normal);
    gl_Position = projectionMatrix * viewMatrix * vec4(vWorldPosition, 1.0);
  }
`;

export const groundFragmentShader = /* glsl */ `
  uniform sampler2D swatch;
  uniform float swatchPeriod;
  uniform float jointWidth;
  uniform float wetness;
  uniform vec3 timeTint;
  uniform vec3 ambientColour;
  uniform vec3 keyColour;
  uniform vec3 keyDirection;
  varying vec2 vUv;
  varying vec2 vCellOrigin;
  varying vec3 vWorldPosition;
  varying vec3 vWorldNormal;

  void main() {
    vec2 worldUv = (vCellOrigin + vUv - vec2(0.5)) / swatchPeriod;
    vec3 base = texture2D(swatch, worldUv).rgb * timeTint;
    float edgeDistance = min(min(vUv.x, 1.0 - vUv.x), min(vUv.y, 1.0 - vUv.y));
    float joint = smoothstep(0.0, jointWidth, edgeDistance);
    base *= mix(0.72, 1.0, joint);
    base = mix(base, base * vec3(0.66, 0.74, 0.82), wetness * 0.32);
    float lambert = max(dot(normalize(vWorldNormal), normalize(keyDirection)), 0.0);
    vec3 lighting = ambientColour + keyColour * lambert;
    gl_FragColor = vec4(base * lighting, 1.0);
  }
`;

const windFieldGlsl = /* glsl */ `
  float windField(vec2 worldXZ, float time) {
    vec2 direction = normalize(windDirection);
    float along = dot(worldXZ, direction);
    float across = dot(worldXZ, vec2(-direction[1], direction[0]));
    float firstOctave = sin(along * 1.37 + across * 0.41 - time * 0.83);
    float secondOctave = sin(along * 2.71 - across * 1.19 - time * 1.43 + 1.8);
    float gust = 0.72 + 0.28 * sin(6.2831853 * time / max(gustPeriod, 0.1) + along * 0.13);
    return (firstOctave * 0.68 + secondOctave * 0.32) * gust;
  }
`;

export const windVertexShader = /* glsl */ `
  uniform float elapsed;
  uniform vec2 windDirection;
  uniform float windStrength;
  uniform float gustPeriod;
  uniform vec2 worldAnchor;
  varying vec2 vUv;
  varying vec2 vWindAnchor;
  varying vec3 vWorldPosition;
  varying vec3 vTangent;
  varying vec3 vBitangent;
  varying vec3 vNormal;

  ${windFieldGlsl}

  void main() {
    vUv = uv;
    vec4 worldPosition;
    vec2 anchor;
    mat3 cardFrame;
    #ifdef USE_INSTANCING
      worldPosition = modelMatrix * instanceMatrix * vec4(position, 1.0);
      vec4 instanceAnchor = modelMatrix * instanceMatrix * vec4(0.0, 0.0, 0.0, 1.0);
      anchor = vec2(instanceAnchor[0], instanceAnchor[2]);
      cardFrame = mat3(modelMatrix) * mat3(instanceMatrix);
    #else
      worldPosition = modelMatrix * vec4(position, 1.0);
      anchor = worldAnchor;
      cardFrame = mat3(modelMatrix);
    #endif
    // The card's own axes in world space. A mirrored card has a negative x
    // scale, so its tangent flips here and a normal sheet mirrors with it.
    vTangent = normalize(cardFrame * vec3(1.0, 0.0, 0.0));
    vBitangent = normalize(cardFrame * vec3(0.0, 1.0, 0.0));
    vNormal = normalize(cardFrame * vec3(0.0, 0.0, 1.0));
    float heightTerm = smoothstep(0.0, 1.0, uv[1]);
    float displacement = windField(anchor, elapsed) * windStrength * 0.075 * heightTerm;
    worldPosition[0] += windDirection[0] * displacement;
    worldPosition[2] += windDirection[1] * displacement;
    vWindAnchor = anchor;
    vWorldPosition = vec3(worldPosition);
    gl_Position = projectionMatrix * viewMatrix * worldPosition;
  }
`;

export const windFragmentShader = /* glsl */ `
  uniform sampler2D albedoTexture;
  uniform sampler2D windWeightTexture;
  #ifdef CARD_NORMAL_MAP
    uniform sampler2D normalTexture;
  #endif
  uniform float elapsed;
  uniform vec2 windDirection;
  uniform float windStrength;
  uniform float gustPeriod;
  uniform vec3 ambientColour;
  uniform vec3 keyColour;
  uniform vec3 keyDirection;
  uniform vec3 lanternPosition;
  uniform vec3 lanternColour;
  uniform float lanternStrength;
  varying vec2 vUv;
  varying vec2 vWindAnchor;
  varying vec3 vWorldPosition;
  varying vec3 vTangent;
  varying vec3 vBitangent;
  varying vec3 vNormal;

  ${windFieldGlsl}
  ${cardWrappedDiffuseGlsl}

  void main() {
    float weight = texture2D(windWeightTexture, vUv)[0];
    vec2 fieldPosition = vWindAnchor + vec2((vUv[0] - 0.5) * 0.7, vUv[1] * 0.5);
    float field = windField(fieldPosition, elapsed);
    vec2 imageDirection = normalize(vec2(
      windDirection[0] * 0.82 + windDirection[1] * 0.48,
      windDirection[1] * 0.18 + 0.02
    ));
    vec2 sampleUv = clamp(
      vUv + imageDirection * field * windStrength * weight * 0.014,
      vec2(0.002),
      vec2(0.998)
    );
    vec4 texel = texture2D(albedoTexture, sampleUv);
    if (texel.a < 0.12) discard;
    vec3 normal = normalize(vNormal);
    #ifdef CARD_NORMAL_MAP
      vec3 sheetNormal = texture2D(normalTexture, sampleUv).xyz * 2.0 - 1.0;
      normal = normalize(
        normalize(vTangent) * sheetNormal.x +
        normalize(vBitangent) * sheetNormal.y +
        normal * sheetNormal.z
      );
    #endif
    vec3 toLantern = lanternPosition - vWorldPosition;
    float lanternDistance = length(toLantern);
    float lanternFalloff = lanternStrength / (1.0 + lanternDistance * lanternDistance * 2.4);
    float lantern = lanternFalloff * cardWrappedDiffuse(normal, toLantern / max(lanternDistance, 0.001));
    vec3 lighting = ambientColour +
      keyColour * cardWrappedDiffuse(normal, keyDirection) +
      lanternColour * lantern;
    gl_FragColor = vec4(texel.rgb * lighting, texel.a);
  }
`;

export const hearthFireVertexShader = /* glsl */ `
  uniform float elapsed;
  varying vec2 vUv;

  void main() {
    vUv = uv;
    vec3 moved = position;
    float crown = smoothstep(0.0, 1.0, uv.y);
    moved.x += (
      sin(elapsed * 2.15 + moved.y * 8.0) * 0.018 +
      sin(elapsed * 3.73 + moved.y * 13.0 + 1.4) * 0.009
    ) * crown;
    moved.y += (
      sin(elapsed * 1.91 + 0.7) * 0.018 +
      sin(elapsed * 3.17 + 2.1) * 0.008
    ) * crown;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(moved, 1.0);
  }
`;

export const hearthFireFragmentShader = /* glsl */ `
  uniform sampler2D albedoTexture;
  uniform float elapsed;
  uniform float flicker;
  varying vec2 vUv;

  void main() {
    float crown = smoothstep(0.0, 1.0, vUv.y);
    vec2 distortion = vec2(
      sin(vUv.y * 19.0 - elapsed * 2.7) +
        sin(vUv.y * 37.0 + elapsed * 4.1 + 1.6),
      sin(vUv.x * 23.0 - elapsed * 3.3) * 0.55
    ) * vec2(0.008, 0.005) * crown;
    vec4 texel = texture2D(albedoTexture, vUv + distortion);
    if (texel.a < 0.12) discard;
    vec3 emissiveColour = texel.rgb + vec3(1.0, 0.31, 0.055) * crown * 0.07 * texel.a;
    gl_FragColor = vec4(emissiveColour * flicker, texel.a);
  }
`;

export const hearthEmberVertexShader = /* glsl */ `
  attribute float emberPhase;
  attribute float emberDrift;
  attribute float emberRise;
  uniform float elapsed;
  uniform vec3 fixtureLateral;
  varying float vLife;
  varying vec2 vUv;

  void main() {
    float age = fract(elapsed / 1.6 + emberPhase);
    vLife = 1.0 - age;
    vUv = uv;
    vec4 worldPosition = modelMatrix * instanceMatrix * vec4(position, 1.0);
    float curl = sin(age * 6.2831853 + emberPhase * 17.0) * emberDrift;
    worldPosition += vec4(fixtureLateral * curl * age, 0.0);
    worldPosition.y += age * emberRise;
    gl_Position = projectionMatrix * viewMatrix * worldPosition;
  }
`;

export const hearthEmberFragmentShader = /* glsl */ `
  varying float vLife;
  varying vec2 vUv;

  void main() {
    float roundness = 1.0 - smoothstep(0.18, 0.5, length(vUv - 0.5));
    gl_FragColor = vec4(vec3(1.0, 0.39, 0.075), vLife * vLife * roundness * 0.72);
  }
`;

export function hearthFlicker(elapsed: number): number {
  return 1 +
    Math.sin(elapsed * 2.17 + 1.731) * 0.08 +
    Math.sin(elapsed * 4.31 + 2.943) * 0.04;
}

export const fogVertexShader = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position, 1.0);
  }
`;

export const fogFragmentShader = /* glsl */ `
  uniform float elapsed;
  uniform vec4 fogColour;
  varying vec2 vUv;

  float hash(vec2 value) {
    return fract(sin(dot(value, vec2(127.1, 311.7))) * 43758.5453);
  }

  void main() {
    vec2 drift = vec2(elapsed * 0.006, elapsed * -0.003);
    float coarse = hash(floor((vUv + drift) * vec2(28.0, 18.0)));
    float veil = mix(0.58, 1.0, coarse);
    gl_FragColor = vec4(fogColour.rgb, fogColour.a * veil);
  }
`;
