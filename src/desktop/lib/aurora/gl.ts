// Aurora's two WebGL renderers.
//
// GlowRenderer draws the light around a frame (the player, the Music page
// card, or a whole monitor): a crisp edge, a hairline highlight just inside
// it, a glow and a wide bloom, each with its own strength and reach. Colours
// flow around the frame from the palette; bass widens the glow, beats lift
// it, highs shimmer along the edge. Only pixels near the edge do any work.
//
// SceneRenderer draws what sits behind the lyrics, on near-black. In the
// Visual styles that is the cover art itself, big and vivid: it pulses with
// the bass, its rows slip sideways and its colour channels split on beats,
// a lens swells it (Fisheye), a larger ghost of it echoes behind, and its
// edges dissolve into the dark like spray paint. Minimal keeps it small and
// dim; Ambient (and songs without a cover) draws slow colour fields from the
// palette instead. A region behind the lyrics is kept calmer so they read.
//
// Both write GLSL ES 1.0 so they run on WebGL 1 and 2.

import type { Levels } from './audio';

const VERT = `
attribute vec2 aPos;
void main() { gl_Position = vec4(aPos, 0.0, 1.0); }
`;

const HEAD = `
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
uniform vec2 uRes;
uniform float uTime;
uniform vec3 uC0, uC1, uC2, uC3;
uniform float uLow, uMid, uHi, uBeat, uLevel;
// 1 = full beat response, 0 = none (no flashes, reduced motion)
uniform float uFlash;
// 0 = still (reduced motion)
uniform float uMotion;

vec3 pal(float t) {
  t = fract(t) * 4.0;
  float f = smoothstep(0.0, 1.0, fract(t));
  if (t < 1.0) return mix(uC0, uC1, f);
  if (t < 2.0) return mix(uC1, uC2, f);
  if (t < 3.0) return mix(uC2, uC3, f);
  return mix(uC3, uC0, f);
}
float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}
`;

const GLOW_FRAG = `${HEAD}
uniform float uDpr, uRadius, uInset, uThick, uEdge, uHigh, uGlow, uGlowSize, uBloom, uBloomSize, uIntensity, uIdle;

float sdRoundBox(vec2 p, vec2 b, float r) {
  vec2 q = abs(p) - b + r;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - r;
}

void main() {
  vec2 c = uRes * 0.5;
  vec2 p = gl_FragCoord.xy - c;
  float r = min(uRadius * uDpr, min(c.x, c.y));
  float e = -sdRoundBox(p, c - uInset * uDpr, r);
  float reach = (uGlowSize * 1.8 + uBloomSize * 1.6) * uDpr + 8.0;
  if (e > reach || e < -1.0) { gl_FragColor = vec4(0.0); return; }

  // Where on the frame (0–1 around), colours flowing along it.
  float ang = atan(p.y / max(c.y, 1.0), p.x / max(c.x, 1.0)) / 6.2831853 + 0.5;
  float flow = uTime * 0.035;
  vec3 col = pal(ang * 2.0 + flow);
  vec3 col2 = pal(ang * 2.0 + flow + 0.5 + 0.12 * sin(uTime * 0.25 + ang * 12.566));

  float beat = uBeat * uFlash;
  float breathe = uIdle * 0.14 * sin(uTime * 0.7);
  float th = uThick * uDpr * (1.0 + 0.3 * uLow);
  // Crisp edge with one pixel of anti-aliasing on each side.
  float edge = clamp(e + 0.5, 0.0, 1.0) * clamp(th - e + 0.5, 0.0, 1.0);
  // A near-white hairline just inside the edge.
  float hl = exp(-abs(e - th - 1.2 * uDpr) / (0.7 * uDpr));
  float gs = uGlowSize * uDpr * (0.75 + 0.7 * uLow + 0.25 * uLevel);
  float glow = exp(-max(e - th, 0.0) / max(gs, 1.0));
  float bs = uBloomSize * uDpr * (0.8 + 0.45 * uLevel + 0.2 * uLow);
  float bloom = exp(-pow(max(e, 0.0) / max(bs, 1.0), 1.35) * 2.0);
  float shimmer = 1.0 + uHi * uMotion * 0.55 * (0.5 + 0.5 * sin(ang * 80.0 - uTime * 3.0));

  float energy = 0.78 + 0.45 * uLevel + 0.3 * beat + breathe;
  vec3 light = col * (uEdge * edge * (1.35 + 0.35 * beat) * shimmer)
    + mix(col, vec3(1.0), 0.4) * (uHigh * hl * (0.8 + 0.3 * uHi))
    + mix(col, col2, 0.5) * (uGlow * glow * energy)
    + col2 * (uBloom * bloom * (0.55 + 0.5 * uLow + 0.25 * beat + breathe));
  light *= uIntensity;
  light = min(light, vec3(1.0));
  float a = max(max(light.r, light.g), light.b);
  gl_FragColor = vec4(light, a);
}
`;

const SCENE_FRAG = `${HEAD}
uniform float uIntensity, uBlur, uSat, uOpacity, uWave, uWaveY, uCalmK, uHasArt, uShow, uGlitch, uLens, uAmbient, uMinimal, uSeed;
uniform vec4 uCalm;
uniform sampler2D uArt, uBands;

float noise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}
float fbm(vec2 p) {
  float v = 0.0, a = 0.5;
  for (int i = 0; i < 4; i++) { v += a * noise(p); p = p * 2.03 + vec2(1.7, 9.2); a *= 0.5; }
  return v;
}
vec2 rot(vec2 p, float a) { float c = cos(a), s = sin(a); return vec2(c * p.x - s * p.y, s * p.x + c * p.y); }

// Slow colour fields (Ambient, or no cover).
vec3 fields(vec2 p, float t) {
  vec2 q = vec2(fbm(p * 1.3 + vec2(t, -t * 0.7)), fbm(p * 1.3 + vec2(-t * 0.8, t * 0.5) + 5.2));
  vec2 w = vec2(fbm(p * 1.5 + 2.0 * q + vec2(t * 1.3, 1.7)), fbm(p * 1.5 + 2.0 * q + vec2(8.3, -t)));
  float f = fbm(p * mix(2.4, 1.1, uBlur) + 2.2 * w);
  vec3 c = mix(uC0, uC1, smoothstep(0.25, 0.75, q.x));
  c = mix(c, uC2, smoothstep(0.35, 0.9, w.y) * 0.8);
  c = mix(c, uC3, smoothstep(0.55, 0.95, f) * 0.6);
  float fold = fbm(vec2(p.x * 2.6 + w.x * 1.8 + t * 0.6, t * 1.4));
  float curtain = pow(fold, 2.2) * smoothstep(-0.55, 0.35, p.y + 0.25 * (q.y - 0.5)) * smoothstep(0.75, 0.05, p.y);
  float e = 0.3 + 0.55 * uLevel + 0.2 * uBeat * uFlash;
  return c * (f * f * f * 0.9 * e + curtain * (0.25 + 0.8 * uMid) * 0.7);
}

// The cover with its effects, and how much of it shows (alpha).
vec4 cover(vec2 p, vec2 frag, float t, float g) {
  // A lens that swells with the bass.
  float r2 = dot(p, p);
  float bulge = uLens * (0.55 + 0.45 * uLow * uMotion);
  p *= 1.0 - bulge * max(0.0, 0.3 - r2) * 1.6;
  vec2 undistorted = p;
  // Rows slipping sideways, blocks jumping, a few rows smeared.
  float rows = mix(12.0, 44.0, hash(vec2(uSeed, 1.3)));
  float row = floor((p.y + 0.5) * rows);
  float slip = step(1.0 - 0.5 * g, hash(vec2(row, uSeed)));
  p.x += slip * (hash(vec2(row, uSeed + 7.0)) - 0.5) * 0.3 * g;
  vec2 blk = floor((p + 0.5) * vec2(5.0, 9.0));
  float jump = step(1.0 - 0.16 * g, hash(blk + uSeed * 1.37));
  p += jump * (vec2(hash(blk + 3.1), hash(blk + 5.7)) - 0.5) * 0.22 * g;
  float smear = step(1.0 - 0.1 * g, hash(vec2(row, uSeed + 3.0)));
  p.x = mix(p.x, floor(p.x * 6.0) / 6.0 + 0.04, smear);
  // Colour channels splitting apart.
  float split = (0.002 + 0.016 * g + 0.006 * uHi * uMotion) * (1.0 - uMinimal);
  vec2 a = p + 0.5;
  float bias = uBlur * 3.0;
  vec3 c;
  c.r = texture2D(uArt, a + vec2(split, split * 0.3), bias).r;
  c.g = texture2D(uArt, a, bias).g;
  c.b = texture2D(uArt, a - vec2(split, split * 0.3), bias).b;
  // Edges dissolve into the dark, grainy like spray paint.
  vec2 q = abs(undistorted);
  float edge = max(q.x, q.y);
  float rough = noise(undistorted * 14.0 + t * 0.5) - 0.5;
  float m = smoothstep(0.5, 0.36, edge + rough * 0.16);
  float grain = step(hash(floor(frag / 1.5) + floor(uTime * 10.0)), m * 1.3);
  m = mix(grain, m, smoothstep(0.55, 0.95, m));
  m *= step(abs(a.x - 0.5), 0.5) * step(abs(a.y - 0.5), 0.5);
  return vec4(c, m);
}

void main() {
  vec2 frag = gl_FragCoord.xy;
  vec2 uv = frag / uRes;
  float aspect = uRes.x / uRes.y;
  vec2 pc = vec2((uv.x - 0.5) * aspect, uv.y - 0.5);
  float t = uTime * 0.05;
  vec3 col = vec3(0.0);

  // A faint glow of the palette behind everything, never more.
  col += uC0 * 0.05 * exp(-dot(pc, pc) * 2.5) * (0.6 + 0.6 * uLevel);

  float artK = uHasArt * (1.0 - uAmbient);
  if (artK > 0.0) {
    float g = uGlitch * uIntensity * uMotion * (1.0 - uMinimal) * (0.15 + 0.85 * uBeat * uFlash + 0.35 * uHi);
    float size = mix(0.94, 0.5, uMinimal) * mix(0.55, 1.0, uShow) * (1.0 + (0.06 * uLow + 0.04 * uBeat * uFlash) * uIntensity * uMotion);
    vec2 p = rot(pc, 0.025 * sin(t * 6.0) * uMotion) / size;
    // A larger ghost behind, breathing out on beats.
    vec4 ghost = cover(p / (1.22 + 0.08 * uBeat * uFlash), frag, t, g * 0.6);
    vec4 art = cover(p, frag, t, g);
    float bright = uOpacity * mix(1.0, 0.45, uMinimal) * (0.78 + 0.3 * uLevel + 0.12 * uBeat * uFlash);
    vec3 ghostCol = mix(ghost.rgb, ghost.rgb * (uC1 * 1.4), 0.5);
    col += ghostCol * ghost.a * 0.13 * (1.0 - uMinimal) * bright;
    col = mix(col, art.rgb * bright, art.a);
  }
  if (uAmbient > 0.0 || uHasArt < 0.5) {
    col += fields(pc, t) * uIntensity * uOpacity * max(uAmbient, 1.0 - uHasArt);
  }

  // The spectrum: a glowing horizon, low notes in the middle.
  if (uWave > 0.5) {
    float x = abs(uv.x - 0.5) * 2.0;
    float band = texture2D(uBands, vec2(0.03 + x * 0.94, 0.5)).r;
    float h = (0.012 + band * 0.09) * (0.6 + 0.4 * uIntensity);
    float dy = uv.y - uWaveY;
    float line = exp(-abs(abs(dy) - h) * uRes.y / 2.5);
    float fill = smoothstep(h, 0.0, abs(dy)) * 0.22;
    col += pal(uv.x * 0.8 + t * 0.4) * (line * 0.75 + fill) * (0.35 + 0.65 * uLevel) * uOpacity * smoothstep(1.0, 0.75, x);
  }

  // Calmer behind the lyrics.
  vec2 d = (uv - uCalm.xy) / max(uCalm.zw, vec2(0.01));
  col *= 1.0 - 0.38 * exp(-dot(d, d) * 0.9) * uCalmK;

  float l = dot(col, vec3(0.299, 0.587, 0.114));
  col = max(mix(vec3(l), col, uSat), 0.0);
  col *= uShow;
  col += (hash(frag + fract(uTime * 7.0)) - 0.5) / 255.0;
  gl_FragColor = vec4(col, 1.0);
}
`;

type GL = WebGLRenderingContext | WebGL2RenderingContext;

function context(canvas: HTMLCanvasElement, transparent: boolean): GL | null {
  const opts: WebGLContextAttributes = {
    alpha: transparent,
    premultipliedAlpha: true,
    antialias: false,
    depth: false,
    stencil: false,
    preserveDrawingBuffer: false,
    powerPreference: 'default',
  };
  return (canvas.getContext('webgl2', opts) as WebGL2RenderingContext | null) ?? (canvas.getContext('webgl', opts) as WebGLRenderingContext | null);
}

function program(gl: GL, frag: string): WebGLProgram {
  const compile = (type: number, src: string) => {
    const s = gl.createShader(type);
    if (!s) throw new Error('WebGL shader');
    gl.shaderSource(s, src);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? 'shader did not compile');
    return s;
  };
  const p = gl.createProgram();
  if (!p) throw new Error('WebGL program');
  gl.attachShader(p, compile(gl.VERTEX_SHADER, VERT));
  gl.attachShader(p, compile(gl.FRAGMENT_SHADER, frag));
  gl.bindAttribLocation(p, 0, 'aPos');
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? 'program did not link');
  // One triangle that covers the screen.
  const buf = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 3, -1, -1, 3]), gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);
  return p;
}

class Base {
  protected gl: GL;
  protected prog: WebGLProgram;
  private loc = new Map<string, WebGLUniformLocation | null>();

  constructor(
    readonly canvas: HTMLCanvasElement,
    frag: string,
    transparent: boolean,
  ) {
    const gl = context(canvas, transparent);
    if (!gl) throw new Error('WebGL is not available');
    this.gl = gl;
    this.prog = program(gl, frag);
    gl.useProgram(this.prog);
  }

  protected u(name: string): WebGLUniformLocation | null {
    if (!this.loc.has(name)) this.loc.set(name, this.gl.getUniformLocation(this.prog, name));
    return this.loc.get(name) ?? null;
  }

  protected f(name: string, v: number) {
    this.gl.uniform1f(this.u(name), v);
  }

  /** Match the drawing buffer to the element (× `scale` device pixels). */
  resize(scale: number): boolean {
    const r = this.canvas.getBoundingClientRect();
    const w = Math.max(1, Math.round(r.width * scale));
    const h = Math.max(1, Math.round(r.height * scale));
    if (this.canvas.width === w && this.canvas.height === h) return false;
    this.canvas.width = w;
    this.canvas.height = h;
    return true;
  }

  protected common(time: number, colors: Float32Array, lv: Levels, flash: number, motion: number) {
    const gl = this.gl;
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.uniform2f(this.u('uRes'), this.canvas.width, this.canvas.height);
    this.f('uTime', time);
    for (let i = 0; i < 4; i++) gl.uniform3f(this.u(`uC${i}`), colors[i * 3], colors[i * 3 + 1], colors[i * 3 + 2]);
    this.f('uLow', lv.low);
    this.f('uMid', lv.mid);
    this.f('uHi', lv.high);
    this.f('uBeat', lv.beat);
    this.f('uLevel', lv.level);
    this.f('uFlash', flash);
    this.f('uMotion', motion);
  }

  get lost(): boolean {
    return this.gl.isContextLost();
  }
}

export interface GlowParams {
  dpr: number;
  radius: number;
  inset: number;
  thickness: number;
  edge: number;
  highlight: number;
  glow: number;
  glowSize: number;
  bloom: number;
  bloomSize: number;
  intensity: number;
  /** 0–1: breathe gently when there is no sound */
  idle: number;
}

export class GlowRenderer extends Base {
  constructor(canvas: HTMLCanvasElement) {
    super(canvas, GLOW_FRAG, true);
  }

  draw(time: number, colors: Float32Array, lv: Levels, flash: number, motion: number, g: GlowParams) {
    const gl = this.gl;
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    this.common(time, colors, lv, flash, motion);
    this.f('uDpr', g.dpr);
    this.f('uRadius', g.radius);
    this.f('uInset', g.inset);
    this.f('uThick', g.thickness);
    this.f('uEdge', g.edge);
    this.f('uHigh', g.highlight);
    this.f('uGlow', g.glow);
    this.f('uGlowSize', g.glowSize);
    this.f('uBloom', g.bloom);
    this.f('uBloomSize', g.bloomSize);
    this.f('uIntensity', g.intensity);
    this.f('uIdle', g.idle);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}

export interface SceneParams {
  intensity: number;
  /** 0–1: softness of the cover (and of the colour fields) */
  blur: number;
  saturation: number;
  /** 0–1: brightness of what is drawn */
  opacity: number;
  /** 0–1: how much the cover glitches */
  glitch: number;
  /** 0–1: fisheye lens */
  lens: number;
  /** 0–1: colour fields instead of the cover */
  ambient: number;
  /** 0 or 1 */
  minimal: number;
  /** 0–1: shown (0 while paused) */
  show: number;
  /** changes a few times a second, picks the glitch pattern */
  seed: number;
  wave: boolean;
  waveY: number;
  /** the lyrics' box: centre x, y and half width, height (0–1, y up) */
  calm: [number, number, number, number];
  calmStrength: number;
}

export class SceneRenderer extends Base {
  private art: WebGLTexture | null;
  private bands: WebGLTexture | null;
  private bandBytes = new Uint8Array(16);

  constructor(canvas: HTMLCanvasElement) {
    super(canvas, SCENE_FRAG, false);
    const gl = this.gl;
    this.art = gl.createTexture();
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.art);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([10, 10, 16, 255]));
    this.params(gl.LINEAR);
    this.bands = gl.createTexture();
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.bands);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.LUMINANCE, 16, 1, 0, gl.LUMINANCE, gl.UNSIGNED_BYTE, this.bandBytes);
    this.params(gl.LINEAR);
    gl.uniform1i(this.u('uArt'), 0);
    gl.uniform1i(this.u('uBands'), 1);
  }

  private params(min: number) {
    const gl = this.gl;
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, min);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  private hasArt = false;

  /** The cover art, sharp (up to 1024², with mipmaps for the softness setting). */
  setArt(img: HTMLImageElement | null) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.art);
    this.hasArt = !!img;
    if (!img) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([0, 0, 0, 255]));
      this.params(gl.LINEAR);
      return;
    }
    // A power of two (WebGL 1 mipmaps need one), no bigger than the cover.
    const side = Math.max(img.naturalWidth, img.naturalHeight) >= 900 ? 1024 : 512;
    const c = document.createElement('canvas');
    c.width = side;
    c.height = side;
    const ctx = c.getContext('2d');
    if (!ctx) return;
    ctx.imageSmoothingQuality = 'high';
    ctx.drawImage(img, 0, 0, side, side);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, c);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.generateMipmap(gl.TEXTURE_2D);
    this.params(gl.LINEAR_MIPMAP_LINEAR);
  }

  draw(time: number, colors: Float32Array, lv: Levels, flash: number, motion: number, s: SceneParams) {
    const gl = this.gl;
    for (let i = 0; i < 16; i++) this.bandBytes[i] = Math.round(Math.min(1, Math.max(0, lv.bands[i])) * 255);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.bands);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 16, 1, gl.LUMINANCE, gl.UNSIGNED_BYTE, this.bandBytes);
    this.common(time, colors, lv, flash, motion);
    this.f('uIntensity', s.intensity);
    this.f('uBlur', s.blur);
    this.f('uSat', s.saturation);
    this.f('uOpacity', s.opacity);
    this.f('uGlitch', s.glitch);
    this.f('uLens', s.lens);
    this.f('uAmbient', s.ambient);
    this.f('uMinimal', s.minimal);
    this.f('uShow', s.show);
    this.f('uSeed', s.seed);
    this.f('uHasArt', this.hasArt ? 1 : 0);
    this.f('uWave', s.wave ? 1 : 0);
    this.f('uWaveY', s.waveY);
    this.f('uCalmK', s.calmStrength);
    gl.uniform4f(this.u('uCalm'), s.calm[0], s.calm[1], s.calm[2], s.calm[3]);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}

/** Whether this machine can draw Aurora with WebGL. */
export function webglAvailable(): boolean {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') ?? c.getContext('webgl'));
  } catch {
    return false;
  }
}
