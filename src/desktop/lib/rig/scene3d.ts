// The PC in 3D, built from what OmniHub detected (motherboard size, cooler,
// memory sticks, graphics card, fans) and what the user told it about the
// case. Its LEDs run the same effect maths as the real lights (effects.ts),
// its fans spin at the speed LibreHardwareMonitor reads, and the cooler's
// LCD and the case screen show the chosen picture or video.
//
// Cheap to draw: plain materials, every LED of a part in one instanced mesh,
// a few point lights for the glow, and drawing only while the page shows.

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import type { LightProfile, RigSettings, Specs } from '@shared/types';
import { colorAt, palette, type Inputs, type Rgb } from './effects';

type Model = RigSettings['model'];

/** A lit part: its LEDs (in order along it) and the effect's device index. */
interface LedGroup {
  mesh: THREE.InstancedMesh;
  dev: number;
  /** Glow planes tinted with the group's average colour. */
  halos: THREE.Mesh[];
}

export type PartId = 'cpu' | 'gpu' | 'ram' | 'board' | 'cooler' | 'fans' | 'psu' | 'screen';

const CM = 1;
const W = 23 * CM;
const H = 48 * CM;
const D = 45 * CM;
const TRAY = -W / 2 + 0.6;

const mat = (color: number, rough = 0.7, metal = 0.2) => new THREE.MeshStandardMaterial({ color, roughness: rough, metalness: metal });

/** The graphics card's length, thickness and fan count from its name. */
export function gpuShape(name: string): { length: number; thick: number; fans: number } {
  const n = name.toUpperCase();
  if (/\b(5090|4090|3090|7900 ?XTX)\b/.test(n)) return { length: 34, thick: 7, fans: 3 };
  if (/\b(5080|4080|3080|7900|9070)\b/.test(n)) return { length: 31, thick: 6.2, fans: 3 };
  if (/\b(5070|4070|3070|7800|7700|9060)\b/.test(n)) return { length: 28, thick: 5.4, fans: n.includes('SUPER') || n.includes('TI') ? 3 : 2 };
  if (/\b(5060|4060|3060|7600|6600)\b/.test(n)) return { length: 24, thick: 4.2, fans: 2 };
  return { length: 26, thick: 5, fans: 2 };
}

export class PcScene {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly camera: THREE.PerspectiveCamera;
  readonly controls: OrbitControls;
  private root = new THREE.Group();
  private groups: LedGroup[] = [];
  private rotors: THREE.Object3D[] = [];
  private screens: THREE.Mesh[] = [];
  private spill: THREE.PointLight[] = [];
  private parts = new Map<PartId, THREE.Object3D[]>();
  private frame = 0;
  private last = performance.now();
  private t0 = performance.now();
  private profile: LightProfile | null = null;
  private colors: Rgb[] = [];
  private inputs: () => Omit<Inputs, 't'> = () => ({ level: 0, beat: 0, temp: null });
  private rpm = 900;
  private media: { url: string; video: boolean } | null = null;
  private mediaTexture: THREE.Texture | null = null;
  private video: HTMLVideoElement | null = null;
  private highlight: PartId | null = null;
  private resize: ResizeObserver;
  private visible = true;
  private onVisibility = () => {
    this.visible = document.visibilityState === 'visible';
    if (this.visible) this.loop();
  };

  constructor(private canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, powerPreference: 'default' });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.camera = new THREE.PerspectiveCamera(32, 1, 1, 600);
    this.camera.position.set(88, 22, 62);
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.controls.dampingFactor = 0.08;
    this.controls.minDistance = 45;
    this.controls.maxDistance = 200;
    this.controls.enablePan = false;
    this.controls.target.set(0, 0, 0);
    this.scene.add(new THREE.HemisphereLight(0xb8c4ff, 0x101018, 0.55));
    const key = new THREE.DirectionalLight(0xffffff, 1.1);
    key.position.set(60, 80, 70);
    this.scene.add(key);
    for (let i = 0; i < 3; i++) {
      const p = new THREE.PointLight(0xffffff, 0, 60, 1.6);
      this.spill.push(p);
      this.scene.add(p);
    }
    this.spill[0].position.set(4, 4, D / 2 - 6);
    this.spill[1].position.set(4, -4, -6);
    this.spill[2].position.set(2, H / 2 - 8, -8);
    this.scene.add(this.root);
    this.resize = new ResizeObserver(() => this.fit());
    this.resize.observe(canvas);
    document.addEventListener('visibilitychange', this.onVisibility);
    this.fit();
    this.loop();
  }

  private fit() {
    const r = this.canvas.getBoundingClientRect();
    const w = Math.max(1, Math.round(r.width));
    const h = Math.max(1, Math.round(r.height));
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
    // Far enough for the whole case, whatever the shape of the view.
    const v = THREE.MathUtils.degToRad(this.camera.fov) / 2;
    const half = Math.min(v, Math.atan(Math.tan(v) * this.camera.aspect));
    const dist = (Math.hypot(W, H, D) / 2 / Math.sin(half)) * 0.92;
    const dir = this.camera.position.clone().sub(this.controls.target).normalize();
    this.camera.position.copy(this.controls.target).addScaledVector(dir, dist);
    this.controls.maxDistance = dist * 1.8;
    this.controls.minDistance = dist * 0.45;
  }

  setLighting(profile: LightProfile | null) {
    if (this.profile?.effect !== profile?.effect) this.t0 = performance.now();
    this.profile = profile;
    this.colors = profile ? palette(profile) : [];
  }

  setInputs(f: () => Omit<Inputs, 't'>) {
    this.inputs = f;
  }

  /** Average fan speed; 0 stops them. */
  setRpm(rpm: number | null) {
    this.rpm = rpm ?? 900;
  }

  setHighlight(part: PartId | null) {
    this.highlight = part;
    for (const [id, objs] of this.parts) {
      for (const o of objs) {
        o.traverse((c) => {
          const m = (c as THREE.Mesh).material as THREE.MeshStandardMaterial | undefined;
          if (m && 'emissive' in m && !(c as THREE.InstancedMesh).isInstancedMesh) m.emissive.setHex(id === part ? 0x223355 : 0x000000);
        });
      }
    }
  }

  /** The picture or video on the cooler's LCD and the case screen. */
  setMedia(media: { url: string; video: boolean } | null) {
    if (media?.url === this.media?.url) return;
    this.media = media;
    this.mediaTexture?.dispose();
    this.mediaTexture = null;
    if (this.video) {
      this.video.pause();
      this.video.removeAttribute('src');
      this.video = null;
    }
    if (media?.video) {
      const v = document.createElement('video');
      v.src = media.url;
      v.muted = true;
      v.loop = true;
      v.playsInline = true;
      void v.play().catch(() => undefined);
      this.video = v;
      this.mediaTexture = new THREE.VideoTexture(v);
    } else if (media) {
      this.mediaTexture = new THREE.TextureLoader().load(media.url);
    }
    if (this.mediaTexture) this.mediaTexture.colorSpace = THREE.SRGBColorSpace;
    for (const s of this.screens) {
      const m = s.material as THREE.MeshBasicMaterial;
      m.map = this.mediaTexture;
      m.color.setHex(this.mediaTexture ? 0xffffff : 0x0b0d16);
      m.needsUpdate = true;
    }
  }

  /** (Re)build the PC from its specs and the user's description. */
  build(specs: Specs | null, model: Model) {
    this.root.traverse((o) => {
      const m = o as THREE.Mesh;
      m.geometry?.dispose();
      const mm = m.material as THREE.Material | THREE.Material[] | undefined;
      if (Array.isArray(mm)) mm.forEach((x) => x.dispose());
      else mm?.dispose();
    });
    this.root.clear();
    this.groups = [];
    this.rotors = [];
    this.screens = [];
    this.parts.clear();
    const add = (id: PartId, o: THREE.Object3D) => {
      this.root.add(o);
      this.parts.set(id, [...(this.parts.get(id) ?? []), o]);
      return o;
    };
    const white = model.caseColor === 'white';
    const shell = mat(white ? 0xe9ebf0 : 0x15161c, 0.55, 0.35);
    const inner = mat(white ? 0xd8dbe2 : 0x0d0e13, 0.8, 0.1);

    // ---- case ----
    const box = (w: number, h: number, d: number, m: THREE.Material, x: number, y: number, z: number) => {
      const b = new THREE.Mesh(new THREE.BoxGeometry(w, h, d), m);
      b.position.set(x, y, z);
      return b;
    };
    const caseGroup = new THREE.Group();
    caseGroup.add(box(0.6, H, D, inner, -W / 2, 0, 0)); // tray side
    caseGroup.add(box(W, 0.6, D, shell, 0, H / 2, 0)); // top
    caseGroup.add(box(W, 0.6, D, shell, 0, -H / 2, 0)); // bottom
    caseGroup.add(box(W, H, 0.6, shell, 0, 0, -D / 2)); // rear
    const front = box(W, H, 0.6, new THREE.MeshStandardMaterial({ color: white ? 0xf2f3f6 : 0x1b1c24, roughness: 0.9, metalness: 0.1, transparent: true, opacity: 0.35 }), 0, 0, D / 2);
    caseGroup.add(front);
    if (model.glass) {
      const glass = new THREE.Mesh(new THREE.BoxGeometry(0.3, H - 1, D - 1), new THREE.MeshStandardMaterial({ color: 0x8fa6c8, roughness: 0.05, metalness: 0, transparent: true, opacity: 0.1, depthWrite: false }));
      glass.position.set(W / 2, 0, 0);
      caseGroup.add(glass);
      const edge = new THREE.LineSegments(new THREE.EdgesGeometry(new THREE.BoxGeometry(0.3, H - 1, D - 1)), new THREE.LineBasicMaterial({ color: 0x5a6680, transparent: true, opacity: 0.6 }));
      edge.position.copy(glass.position);
      caseGroup.add(edge);
    } else {
      caseGroup.add(box(0.6, H, D, shell, W / 2, 0, 0));
    }
    // Feet.
    for (const [x, z] of [[-7, -16], [7, -16], [-7, 16], [7, 16]]) caseGroup.add(box(3, 1.2, 4, inner, x, -H / 2 - 0.9, z));
    this.root.add(caseGroup);

    // ---- PSU shroud ----
    const shroud = new THREE.Group();
    shroud.add(box(W - 1.2, 0.5, D - 1.2, inner, 0, -H / 2 + 9, 0));
    shroud.add(box(0.5, 9, D - 1.2, inner, W / 2 - 1, -H / 2 + 4.5, 0));
    const psuLabel = box(0.2, 3, 12, mat(0x2b2f3a, 0.6, 0.4), W / 2 - 0.7, -H / 2 + 4.5, -10);
    shroud.add(psuLabel);
    add('psu', shroud);

    // ---- motherboard ----
    const size = specs?.boardSize ?? 'atx';
    const [bh, bw] = size === 'miniItx' ? [17, 17] : size === 'microAtx' ? [24.4, 24.4] : size === 'eAtx' ? [33, 30.5] : [30.5, 24.4];
    const top = H / 2 - 3.5;
    const rear = -D / 2 + 2.5;
    const boardY = top - bh / 2;
    const boardZ = rear + bw / 2;
    const board = new THREE.Group();
    board.add(box(0.2, bh, bw, mat(0x111820, 0.8, 0.2), TRAY + 0.5, boardY, boardZ));
    // VRM heatsinks and the I/O cover.
    board.add(box(1.6, 2.4, bw * 0.55, mat(0x3a3f4b, 0.4, 0.8), TRAY + 1.4, top - 2, rear + bw * 0.32));
    board.add(box(1.6, bh * 0.35, 2.4, mat(0x3a3f4b, 0.4, 0.8), TRAY + 1.4, top - bh * 0.2, rear + 1.5));
    board.add(box(2.4, bh * 0.3, 3.4, mat(0x22252d, 0.5, 0.6), TRAY + 1.8, top - bh * 0.18, rear + 2.4));
    // Chipset and M.2 covers.
    board.add(box(0.8, 5, 6, mat(0x2d313b, 0.4, 0.7), TRAY + 0.9, boardY - bh * 0.28, boardZ + bw * 0.22));
    board.add(box(0.5, 2.4, 9, mat(0x2d313b, 0.4, 0.7), TRAY + 0.8, boardY - 1, boardZ));
    add('board', board);
    const boardLeds = this.ledLine(10, (i, n) => new THREE.Vector3(TRAY + 3.1, top - bh * 0.05 - (i / (n - 1)) * bh * 0.32, rear + 3.7), 0.5, 0);
    add('board', boardLeds.mesh);

    // ---- CPU and cooler ----
    const cpuY = top - Math.min(9, bh * 0.3);
    const cpuZ = rear + Math.min(10.5, bw * 0.43);
    const coolerKind = model.cooler ?? (specs?.cooler === 'aio' ? 'aio' : 'air');
    const cooler = new THREE.Group();
    if (coolerKind === 'aio') {
      const pump = new THREE.Mesh(new THREE.CylinderGeometry(3.4, 3.4, 3.4, 40), mat(0x1a1c22, 0.4, 0.6));
      pump.rotation.z = Math.PI / 2;
      pump.position.set(TRAY + 2.4, cpuY, cpuZ);
      cooler.add(pump);
      const lcd = new THREE.Mesh(new THREE.CircleGeometry(2.8, 48), new THREE.MeshBasicMaterial({ color: 0x0b0d16, map: this.mediaTexture }));
      lcd.rotation.y = Math.PI / 2;
      lcd.position.set(TRAY + 4.15, cpuY, cpuZ);
      cooler.add(lcd);
      this.screens.push(lcd);
      const ring = this.ledRing(20, 3.25, new THREE.Vector3(TRAY + 4.12, cpuY, cpuZ), 'x', 0.32, 3);
      add('cooler', ring.mesh);
      // Radiator: on top when the top has room for it, else at the front.
      const mm = model.radiatorMm ?? specs?.radiatorMm ?? 360;
      const fansOn = Math.max(1, Math.round(mm / 120));
      const onTop = model.topFans >= fansOn;
      const rad = onTop ? box(12.4, 2.7, fansOn * 12, mat(0x22242b, 0.6, 0.5), -1, H / 2 - 4, D / 2 - 3 - (fansOn * 12) / 2) : box(12.4, fansOn * 12, 2.7, mat(0x22242b, 0.6, 0.5), -1, H / 2 - 4 - (fansOn * 12) / 2, D / 2 - 4.5);
      cooler.add(rad);
      // Tubes from the pump to the radiator's end.
      const end = onTop ? new THREE.Vector3(-1, H / 2 - 5.5, D / 2 - 5) : new THREE.Vector3(-1, H / 2 - 6, D / 2 - 6);
      for (const dz of [-1.2, 1.2]) {
        const a = new THREE.Vector3(TRAY + 3, cpuY + 2.6, cpuZ + dz);
        const curve = new THREE.CubicBezierCurve3(a, a.clone().add(new THREE.Vector3(4, 6, 0)), end.clone().add(new THREE.Vector3(2, -4, -6 + dz)), end.clone().add(new THREE.Vector3(0, 0, dz)));
        cooler.add(new THREE.Mesh(new THREE.TubeGeometry(curve, 24, 0.55, 8), mat(0x101115, 0.9, 0)));
      }
    } else {
      const sink = box(15, 12.5, 6.5, mat(0xa9afba, 0.35, 0.9), TRAY + 8, cpuY, cpuZ);
      cooler.add(sink);
      for (let i = 0; i < 22; i++) cooler.add(box(15.2, 0.12, 6.6, mat(0x7a8190, 0.4, 0.9), TRAY + 8, cpuY - 6 + (i * 12) / 21, cpuZ));
      cooler.add(box(15.4, 0.8, 6.8, mat(0x202228, 0.4, 0.6), TRAY + 8, cpuY + 6.6, cpuZ));
      this.fan(new THREE.Vector3(TRAY + 8, cpuY, cpuZ + 4.2), 'z', 12, 3, cooler);
      const cap = this.ledLine(12, (i, n) => new THREE.Vector3(TRAY + 1.5 + (i / (n - 1)) * 13, cpuY + 7.1, cpuZ), 0.4, 3);
      add('cooler', cap.mesh);
    }
    add('cooler', cooler);
    add('cpu', box(0.4, 4, 4, mat(0x8a8f99, 0.3, 0.9), TRAY + 0.8, cpuY, cpuZ));

    // ---- memory ----
    const sticks = Math.max(1, Math.min(4, specs?.ram.length || 2));
    const slots = sticks === 1 ? [1] : sticks === 2 ? [1, 3] : sticks === 3 ? [0, 1, 3] : [0, 1, 2, 3];
    const ram = new THREE.Group();
    const ramLeds: THREE.Vector3[] = [];
    for (const s of slots) {
      const z = cpuZ + 5.6 + s * 0.85;
      ram.add(box(3.6, 13.3, 0.7, mat(0x1d2028, 0.5, 0.6), TRAY + 2.3, cpuY - 1.5, z));
      if (model.ramRgb) for (let k = 0; k < 8; k++) ramLeds.push(new THREE.Vector3(TRAY + 4.3, cpuY - 7.5 + (k / 7) * 12, z));
    }
    add('ram', ram);
    if (ramLeds.length) add('ram', this.ledPoints(ramLeds, 0.42, 1).mesh);

    // ---- graphics card ----
    const gpu = gpuShape(specs?.gpus[0] ?? '');
    const slotY = boardY + bh / 2 - Math.min(16, bh * 0.52);
    const card = new THREE.Group();
    const gpuBody = mat(0x1b1d24, 0.45, 0.6);
    const gpuX = TRAY + 1 + 6.6;
    if (model.verticalGpu) {
      card.add(box(gpu.thick, gpu.length * 0.95, 12.5, gpuBody, W / 2 - gpu.thick / 2 - 2.5, -H / 2 + 9 + (gpu.length * 0.95) / 2 + 1, rear + 7.5));
      for (let f = 0; f < gpu.fans; f++) this.fan(new THREE.Vector3(W / 2 - gpu.thick - 2.6, -H / 2 + 12 + f * 9.4, rear + 7.5), 'x', 8.6, 2, card, true);
      const strip = this.ledLine(16, (i, n) => new THREE.Vector3(W / 2 - 2.4, -H / 2 + 10.5 + (i / (n - 1)) * gpu.length * 0.9, rear + 1.3), 0.5, 2);
      add('gpu', strip.mesh);
    } else {
      card.add(box(13, gpu.thick, gpu.length, gpuBody, gpuX, slotY - gpu.thick / 2 - 0.6, rear + gpu.length / 2 + 0.5));
      card.add(box(13.2, 0.25, gpu.length - 1, mat(0x30343e, 0.3, 0.9), gpuX, slotY - 0.4, rear + gpu.length / 2 + 0.5)); // backplate
      for (let f = 0; f < gpu.fans; f++) this.fan(new THREE.Vector3(gpuX, slotY - gpu.thick - 0.7, rear + 5.5 + f * ((gpu.length - 9) / Math.max(1, gpu.fans - 1))), 'y', 8.6, 2, card, true);
      const strip = this.ledLine(18, (i, n) => new THREE.Vector3(gpuX + 6.6, slotY - 1.4, rear + 3 + (i / (n - 1)) * (gpu.length - 5)), 0.45, 2);
      add('gpu', strip.mesh);
    }
    add('gpu', card);

    // ---- case fans ----
    const fans = new THREE.Group();
    let dev = 4;
    for (let i = 0; i < model.frontFans; i++) this.fan(new THREE.Vector3(-1, H / 2 - 10 - i * 12.6, D / 2 - 1.6), 'z', 12, dev++, fans);
    for (let i = 0; i < model.topFans; i++) this.fan(new THREE.Vector3(-1, H / 2 - 1.6, D / 2 - 9 - i * 12.6), 'y', 12, dev++, fans);
    for (let i = 0; i < model.rearFans; i++) this.fan(new THREE.Vector3(-1, top - 6.5, -D / 2 + 1.6), 'z', 12, dev++, fans);
    for (let i = 0; i < model.bottomFans; i++) this.fan(new THREE.Vector3(-1, -H / 2 + 10.6, D / 2 - 9 - i * 12.6), 'y', 12, dev++, fans);
    add('fans', fans);

    // ---- case screen ----
    if (model.caseScreen) {
      const frame = box(0.6, 12.6, 21, mat(0x0c0d12, 0.5, 0.4), W / 2 - 2.2, -H / 2 + 16.5, D / 2 - 13);
      const panel = new THREE.Mesh(new THREE.PlaneGeometry(20, 11.6), new THREE.MeshBasicMaterial({ color: this.mediaTexture ? 0xffffff : 0x0b0d16, map: this.mediaTexture }));
      panel.rotation.y = Math.PI / 2;
      panel.position.set(W / 2 - 1.85, -H / 2 + 16.5, D / 2 - 13);
      this.screens.push(panel);
      add('screen', frame);
      add('screen', panel);
    }
    this.setHighlight(this.highlight);
  }

  /** LEDs at given points. */
  private ledPoints(points: THREE.Vector3[], size: number, dev: number): LedGroup {
    const mesh = new THREE.InstancedMesh(new THREE.SphereGeometry(size, 8, 6), new THREE.MeshBasicMaterial({ color: 0xffffff, toneMapped: false }), points.length);
    const m = new THREE.Matrix4();
    points.forEach((p, i) => {
      mesh.setMatrixAt(i, m.makeTranslation(p.x, p.y, p.z));
      mesh.setColorAt(i, new THREE.Color(0, 0, 0));
    });
    const g = { mesh, dev, halos: [] };
    this.groups.push(g);
    return g;
  }

  private ledLine(n: number, at: (i: number, n: number) => THREE.Vector3, size: number, dev: number): LedGroup {
    return this.ledPoints(Array.from({ length: n }, (_, i) => at(i, n)), size, dev);
  }

  private ledRing(n: number, r: number, c: THREE.Vector3, axis: 'x' | 'y' | 'z', size: number, dev: number): LedGroup {
    const pts = Array.from({ length: n }, (_, i) => {
      const a = (i / n) * Math.PI * 2;
      const [u, v] = [Math.cos(a) * r, Math.sin(a) * r];
      return axis === 'x' ? new THREE.Vector3(c.x, c.y + u, c.z + v) : axis === 'y' ? new THREE.Vector3(c.x + u, c.y, c.z + v) : new THREE.Vector3(c.x + u, c.y + v, c.z);
    });
    return this.ledPoints(pts, size, dev);
  }

  /** A fan facing along `axis`, with an LED ring and a glow. */
  private fan(c: THREE.Vector3, axis: 'x' | 'y' | 'z', size: number, dev: number, into: THREE.Group, quiet = false) {
    const g = new THREE.Group();
    g.position.copy(c);
    if (axis === 'x') g.rotation.y = Math.PI / 2;
    if (axis === 'y') g.rotation.x = Math.PI / 2;
    const frame = new THREE.Mesh(new THREE.TorusGeometry(size / 2 - 0.5, 0.5, 8, 40), mat(0x16171d, 0.6, 0.3));
    g.add(frame);
    const rotor = new THREE.Group();
    const blade = new THREE.MeshStandardMaterial({ color: quiet ? 0x23252c : 0x2a2d36, roughness: 0.5, metalness: 0.1, transparent: true, opacity: 0.92 });
    for (let i = 0; i < (quiet ? 9 : 7); i++) {
      const b = new THREE.Mesh(new THREE.BoxGeometry(size * 0.36, size * 0.13, 0.12), blade);
      b.position.x = size * 0.22;
      b.rotation.x = 0.45;
      const arm = new THREE.Group();
      arm.rotation.z = (i / (quiet ? 9 : 7)) * Math.PI * 2;
      arm.add(b);
      rotor.add(arm);
    }
    rotor.add(new THREE.Mesh(new THREE.CylinderGeometry(size * 0.14, size * 0.14, 0.9, 24).rotateX(Math.PI / 2), mat(0x15161b, 0.5, 0.4)));
    g.add(rotor);
    this.rotors.push(rotor);
    into.add(g);
    if (quiet) return;
    // The ring's LEDs, in world space.
    g.updateMatrixWorld(true);
    const pts = Array.from({ length: 16 }, (_, i) => {
      const a = (i / 16) * Math.PI * 2;
      return new THREE.Vector3(Math.cos(a) * (size / 2 - 0.5), Math.sin(a) * (size / 2 - 0.5), 0.55).applyEuler(g.rotation).add(c);
    });
    const ring = this.ledPoints(pts, 0.38, dev);
    this.root.add(ring.mesh);
    const halo = new THREE.Mesh(new THREE.RingGeometry(size * 0.18, size * 0.5, 40), new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.55, blending: THREE.AdditiveBlending, depthWrite: false, side: THREE.DoubleSide, toneMapped: false }));
    halo.position.set(0, 0, 0.35);
    g.add(halo);
    ring.halos.push(halo);
  }

  private loop = () => {
    cancelAnimationFrame(this.frame);
    if (!this.visible) return;
    this.frame = requestAnimationFrame(this.loop);
    const now = performance.now();
    // Full refresh rate while you look at it; ~30 fps behind a game or another app.
    if (!document.hasFocus() && now - this.last < 32) return;
    const dt = Math.min(0.1, (now - this.last) / 1000);
    this.last = now;
    this.controls.update();
    const spin = (this.rpm / 60) * Math.PI * 2 * 0.12 * dt;
    for (const r of this.rotors) r.rotation.z -= spin;
    this.paint((now - this.t0) / 1000);
    if (this.mediaTexture instanceof THREE.VideoTexture) this.mediaTexture.needsUpdate = true;
    this.renderer.render(this.scene, this.camera);
  };

  private tmp = new THREE.Color();

  private paint(t: number) {
    const p = this.profile;
    const inp = { ...this.inputs(), t };
    const sums: [number, number, number, number][] = [
      [0, 0, 0, 0],
      [0, 0, 0, 0],
      [0, 0, 0, 0],
    ];
    for (const g of this.groups) {
      const n = g.mesh.count;
      let r = 0;
      let gr = 0;
      let b = 0;
      for (let i = 0; i < n; i++) {
        const c = p ? colorAt(p, this.colors, n > 1 ? i / (n - 1) : 0, g.dev, inp) : ([18, 20, 28] as Rgb);
        r += c[0];
        gr += c[1];
        b += c[2];
        g.mesh.setColorAt(i, this.tmp.setRGB(c[0] / 255, c[1] / 255, c[2] / 255, THREE.SRGBColorSpace));
      }
      if (g.mesh.instanceColor) g.mesh.instanceColor.needsUpdate = true;
      const avg = this.tmp.setRGB(r / n / 255, gr / n / 255, b / n / 255, THREE.SRGBColorSpace);
      for (const h of g.halos) (h.material as THREE.MeshBasicMaterial).color.copy(avg).multiplyScalar(0.7);
      const slot = g.dev >= 4 ? 0 : g.dev === 2 ? 1 : 2;
      sums[slot][0] += r / n;
      sums[slot][1] += gr / n;
      sums[slot][2] += b / n;
      sums[slot][3] += 1;
    }
    sums.forEach(([r, g, b, k], i) => {
      const l = this.spill[i];
      if (!k) {
        l.intensity = 0;
        return;
      }
      l.color.setRGB(r / k / 255, g / k / 255, b / k / 255, THREE.SRGBColorSpace);
      l.intensity = (Math.max(r, g, b) / k / 255) * 900;
    });
  }

  dispose() {
    cancelAnimationFrame(this.frame);
    document.removeEventListener('visibilitychange', this.onVisibility);
    this.resize.disconnect();
    this.controls.dispose();
    this.setMedia(null);
    this.build(null, { caseColor: 'black', glass: false, cooler: 'air', radiatorMm: null, frontFans: 0, topFans: 0, rearFans: 0, bottomFans: 0, ramRgb: false, verticalGpu: false, caseScreen: false });
    this.renderer.dispose();
  }
}
