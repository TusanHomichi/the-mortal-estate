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

export const swayVertexShader = /* glsl */ `
  uniform float elapsed;
  uniform float windStrength;
  uniform float timeOffset;
  varying vec2 vUv;
  varying vec3 vWorldPosition;

  void main() {
    vUv = uv;
    vec3 moved = position;
    float crown = smoothstep(0.18, 1.0, 1.0 - uv.y);
    moved.x += sin(elapsed * 1.15 + timeOffset + moved.y * 1.7) * windStrength * 0.055 * crown;
    vec4 worldPosition = modelMatrix * vec4(moved, 1.0);
    vWorldPosition = vec3(worldPosition);
    gl_Position = projectionMatrix * viewMatrix * worldPosition;
  }
`;

export const swayFragmentShader = /* glsl */ `
  uniform sampler2D albedoTexture;
  uniform vec3 ambientColour;
  uniform vec3 keyColour;
  uniform vec3 lanternPosition;
  uniform vec3 lanternColour;
  uniform float lanternStrength;
  varying vec2 vUv;
  varying vec3 vWorldPosition;

  void main() {
    vec4 texel = texture2D(albedoTexture, vUv);
    if (texel.a < 0.12) discard;
    float lanternDistance = length(vWorldPosition - lanternPosition);
    float lantern = lanternStrength / (1.0 + lanternDistance * lanternDistance * 2.4);
    vec3 lighting = ambientColour + keyColour * 0.48 + lanternColour * lantern;
    gl_FragColor = vec4(texel.rgb * lighting, texel.a);
  }
`;

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
