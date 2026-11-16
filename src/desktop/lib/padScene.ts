// A controller in 3D for the button studio: an original model for each
// family (Xbox-style offset sticks, PlayStation-style side-by-side sticks
// with a touch pad and light bar, Nintendo-style), built from simple shapes.
// Pressed buttons light up and sink, sticks tilt, triggers pull. Click a part
// to pick it. Draws only while visible.

import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLYPHS, type PadFamily } from './pads';

/** Buttons in the browser's standard layout. */
export const BUTTON_NAMES = ['Bottom face', 'Right face', 'Left face', 'Top face', 'Left bumper', 'Right bumper', 'Left trigger', 'Right trigger', 'View / Create / −', 'Menu / Options / +', 'Left stick click', 'Right stick click', 'D-pad up', 'D-pad down', 'D-pad left', 'D-pad right', 'Home'];

export function buttonLabel(family: PadFamily, i: number): string {
  const g = GLYPHS[family];
  const face = [g.bottom, g.right, g.left, g.top];
  if (i < 4) return face[i];
  return [g.lb, g.rb, family === 'playstation' ? 'L2' : family === 'nintendo' ? 'ZL' : 'LT', family === 'playstation' ? 'R2' : family === 'nintendo' ? 'ZR' : 'RT', g.view, g.start, 'L3', 'R3', '↑', '↓', '←', '→', 'Home'][i - 4] ?? `${i}`;
}

interface Part {
  mesh: THREE.Mesh;
  rest: THREE.Vector3;
  mat: THREE.MeshStandardMaterial;
  base: THREE.Color;
  /** How it moves when pressed: sink, or turn (triggers). */
  kind: 'sink' | 'turn';
}

const glyphTexture = (text: string, color: string) => {
  const c = document.createElement('canvas');
  c.width = c.height = 128;
  const g = c.getContext('2d')!;
  g.fillStyle = '#1b1b22';
  g.beginPath();
  g.arc(64, 64, 64, 0, Math.PI * 2);
  g.fill();
  g.fillStyle = color;
  g.font = `bold ${text.length > 1 ? 50 : 72}px system-ui, sans-serif`;
  g.textAlign = 'center';
  g.textBaseline = 'middle';
  g.fillText(text, 64, 68);
  const t = new THREE.CanvasTexture(c);
  t.colorSpace = THREE.SRGBColorSpace;
  return t;
};

/** The body outline (top view): two grips and a bridge, in cm. */
function bodyShape(family: PadFamily): THREE.Shape {
  const s = new THREE.Shape();
  const w = family === 'playstation' ? 8.2 : 8.6;
  s.moveTo(-w, 3.4);
  s.bezierCurveTo(-w + 2.2, 4.6, w - 2.2, 4.6, w, 3.4);
  s.bezierCurveTo(w + 2.4, 2.4, w + 3.6, -3.5, w + 2.6, -6.6);
  s.bezierCurveTo(w + 1.8, -8.6, w - 1.2, -8.4, w - 2.4, -6.2);
  s.bezierCurveTo(w - 3.4, -4.4, w - 4.6, -3.2, 0, -3.2);
  s.bezierCurveTo(-w + 4.6, -3.2, -w + 3.4, -4.4, -w + 2.4, -6.2);
  s.bezierCurveTo(-w + 1.2, -8.4, -w - 1.8, -8.6, -w - 2.6, -6.6);
  s.bezierCurveTo(-w - 3.6, -3.5, -w - 2.4, 2.4, -w, 3.4);
  return s;
}

export class PadScene {
  readonly renderer: THREE.WebGLRenderer;
  readonly scene = new THREE.Scene();
  readonly camera: THREE.PerspectiveCamera;
  readonly controls: OrbitControls;
  private root = new THREE.Group();
  private parts = new Map<number, Part>();
  private sticks: [THREE.Object3D | null, THREE.Object3D | null] = [null, null];
  private body: THREE.MeshStandardMaterial | null = null;
  private trim: THREE.MeshStandardMaterial[] = [];
  private lightBar: THREE.MeshStandardMaterial | null = null;
  private selected: number | null = null;
  private frame = 0;
  private resize: ResizeObserver;
  private ray = new THREE.Raycaster();
  private pointer = new THREE.Vector2();
  private down = { x: 0, y: 0 };
  onPick: (button: number) => void = () => undefined;

  constructor(private canvas: HTMLCanvasElement) {
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;
    this.camera = new THREE.PerspectiveCamera(30, 1, 1, 200);
    this.camera.position.set(0, 40, 40);
    this.controls = new OrbitControls(this.camera, canvas);
    this.controls.enableDamping = true;
    this.controls.enablePan = false;
    this.controls.minDistance = 30;
    this.controls.maxDistance = 90;
    this.controls.target.set(0, 0, -1.5);
    this.scene.add(new THREE.HemisphereLight(0xffffff, 0x223344, 1.4));
    const key = new THREE.DirectionalLight(0xffffff, 2.2);
    key.position.set(10, 22, 14);
    this.scene.add(key);
    const rim = new THREE.DirectionalLight(0x99bbff, 1.1);
    rim.position.set(-14, 8, -18);
    this.scene.add(rim);
    this.scene.add(this.root);
    this.resize = new ResizeObserver(() => this.fit());
    this.resize.observe(canvas);
    canvas.addEventListener('pointerdown', this.onDown);
    canvas.addEventListener('pointerup', this.onUp);
    this.fit();
    this.loop();
  }

  private onDown = (e: PointerEvent) => {
    this.down = { x: e.clientX, y: e.clientY };
  };

  /** A click (not a drag) picks the part under the pointer. */
  private onUp = (e: PointerEvent) => {
    if (Math.hypot(e.clientX - this.down.x, e.clientY - this.down.y) > 5) return;
    const r = this.canvas.getBoundingClientRect();
    this.pointer.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
    this.ray.setFromCamera(this.pointer, this.camera);
    const hit = this.ray.intersectObjects([...this.parts.values()].map((p) => p.mesh), false)[0];
    const b = hit?.object.userData.button;
    if (typeof b === 'number') this.onPick(b);
  };

  private fit() {
    const w = this.canvas.clientWidth || 1;
    const h = this.canvas.clientHeight || 1;
    this.renderer.setSize(w, h, false);
    this.camera.aspect = w / h;
    this.camera.updateProjectionMatrix();
  }

  private clear() {
    this.root.traverse((o) => {
      const m = o as THREE.Mesh;
      m.geometry?.dispose();
      const mm = m.material as THREE.Material | THREE.Material[] | undefined;
      if (Array.isArray(mm)) mm.forEach((x) => x.dispose());
      else mm?.dispose();
    });
    this.root.clear();
    this.parts.clear();
    this.trim = [];
    this.sticks = [null, null];
    this.lightBar = null;
  }

  /** Build the model for a family, in the chosen colours. */
  build(family: PadFamily, colors: { body: string; trim: string; light?: string }) {
    this.clear();
    const g = GLYPHS[family];
    this.body = new THREE.MeshStandardMaterial({ color: colors.body, roughness: 0.55, metalness: 0.08 });
    const trimMat = () => {
      const m = new THREE.MeshStandardMaterial({ color: colors.trim, roughness: 0.45, metalness: 0.15 });
      this.trim.push(m);
      return m;
    };
    const shell = new THREE.Mesh(new THREE.ExtrudeGeometry(bodyShape(family), { depth: 2.4, bevelEnabled: true, bevelThickness: 0.9, bevelSize: 0.9, bevelSegments: 5, curveSegments: 28 }), this.body);
    shell.rotation.x = -Math.PI / 2;
    shell.position.y = -1.6;
    this.root.add(shell);

    const top = 1.8;
    const add = (button: number, geo: THREE.BufferGeometry, mat: THREE.MeshStandardMaterial, pos: [number, number, number], kind: Part['kind'] = 'sink', rot?: [number, number, number]) => {
      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(...pos);
      if (rot) mesh.rotation.set(...rot);
      mesh.userData.button = button;
      this.root.add(mesh);
      this.parts.set(button, { mesh, rest: mesh.position.clone(), mat, base: mat.color.clone(), kind });
      return mesh;
    };

    // Where things go: Xbox and Nintendo offset the sticks, PlayStation puts them side by side.
    const ps = family === 'playstation';
    const lStick: [number, number] = ps ? [-3.4, 0.9] : [-6.4, -1.2];
    const dpad: [number, number] = ps ? [-6.8, -1.6] : [-3.6, 1.9];
    const face: [number, number] = [6.6, -1.4];
    const rStick: [number, number] = ps ? [3.4, 0.9] : [3.6, 1.9];

    // Face buttons with their symbols.
    const faceAt: [number, number][] = [
      [0, 1.55],
      [1.55, 0],
      [-1.55, 0],
      [0, -1.55],
    ];
    const symbols = [g.bottom, g.right, g.left, g.top];
    faceAt.forEach(([dx, dz], i) => {
      const color = g.colors?.[i] ?? '#f2f2f2';
      const mats = [new THREE.MeshStandardMaterial({ color: '#202028', roughness: 0.4 }), new THREE.MeshStandardMaterial({ map: glyphTexture(symbols[i], color), roughness: 0.35 }), new THREE.MeshStandardMaterial({ color: '#202028' })];
      const mesh = new THREE.Mesh(new THREE.CylinderGeometry(0.62, 0.62, 0.5, 28), mats);
      mesh.position.set(face[0] + dx, top + 0.25, face[1] + dz);
      mesh.rotation.y = Math.PI;
      mesh.userData.button = i;
      this.root.add(mesh);
      this.parts.set(i, { mesh, rest: mesh.position.clone(), mat: mats[1], base: new THREE.Color('#ffffff'), kind: 'sink' });
    });

    // D-pad: four arms.
    const arm = new THREE.BoxGeometry(0.85, 0.42, 1.2);
    ([
      [12, 0, -1],
      [13, 0, 1],
      [14, -1, 0],
      [15, 1, 0],
    ] as [number, number, number][]).forEach(([b, dx, dz]) => {
      const m = add(b, arm.clone(), trimMat(), [dpad[0] + dx * 0.95, top + 0.2, dpad[1] + dz * 0.95]);
      if (dx) m.rotation.y = Math.PI / 2;
    });
    arm.dispose();

    // Sticks: a base, a shaft and a cap that tilt together.
    const stick = (button: number, at: [number, number], idx: 0 | 1) => {
      const pivot = new THREE.Group();
      pivot.position.set(at[0], top, at[1]);
      const well = new THREE.Mesh(new THREE.TorusGeometry(1.25, 0.18, 10, 30), new THREE.MeshStandardMaterial({ color: '#15151b', roughness: 0.6 }));
      well.rotation.x = Math.PI / 2;
      well.position.set(at[0], top - 0.05, at[1]);
      this.root.add(well);
      const shaft = new THREE.Mesh(new THREE.CylinderGeometry(0.35, 0.45, 0.9, 16), new THREE.MeshStandardMaterial({ color: '#202028' }));
      shaft.position.y = 0.45;
      pivot.add(shaft);
      const capMat = trimMat();
      const cap = new THREE.Mesh(new THREE.CylinderGeometry(1.05, 1.0, 0.42, 30), capMat);
      cap.position.y = 1.05;
      cap.userData.button = button;
      pivot.add(cap);
      this.root.add(pivot);
      this.parts.set(button, { mesh: cap, rest: cap.position.clone(), mat: capMat, base: capMat.color.clone(), kind: 'sink' });
      this.sticks[idx] = pivot;
    };
    stick(10, lStick, 0);
    stick(11, rStick, 1);

    // Middle buttons: view, menu, home.
    add(8, new THREE.CylinderGeometry(0.38, 0.38, 0.3, 20), trimMat(), [-1.7, top + 0.1, -0.9]);
    add(9, new THREE.CylinderGeometry(0.38, 0.38, 0.3, 20), trimMat(), [1.7, top + 0.1, -0.9]);
    add(16, new THREE.CylinderGeometry(0.62, 0.62, 0.3, 24), new THREE.MeshStandardMaterial({ color: family === 'xbox' ? '#e6e6e6' : '#26262e', roughness: 0.3 }), [0, top + 0.1, ps ? 2.3 : 0.4]);

    // PlayStation: touch pad and light bar.
    if (ps) {
      const pad = new THREE.Mesh(new THREE.BoxGeometry(5.6, 0.25, 3.2), new THREE.MeshStandardMaterial({ color: '#18181e', roughness: 0.35, metalness: 0.1 }));
      pad.position.set(0, top + 0.05, -2.2);
      this.root.add(pad);
      this.lightBar = new THREE.MeshStandardMaterial({ color: '#111', emissive: colors.light ?? '#3b82f6', emissiveIntensity: 1.6 });
      const bar = new THREE.Mesh(new THREE.BoxGeometry(5.4, 0.18, 0.18), this.lightBar);
      bar.position.set(0, top + 0.2, -3.95);
      this.root.add(bar);
    }

    // Shoulders: bumpers on top, triggers behind them.
    const bumper = new THREE.BoxGeometry(4.2, 0.7, 1.0);
    add(4, bumper.clone(), trimMat(), [-6.6, top - 0.5, -4.6]);
    add(5, bumper.clone(), trimMat(), [6.6, top - 0.5, -4.6]);
    bumper.dispose();
    const trigger = new THREE.BoxGeometry(2.6, 1.6, 1.4);
    add(6, trigger.clone(), trimMat(), [-6.4, top - 0.9, -5.8], 'turn');
    add(7, trigger.clone(), trimMat(), [6.4, top - 0.9, -5.8], 'turn');
    trigger.dispose();
    this.select(this.selected);
  }

  recolor(colors: { body?: string; trim?: string; light?: string }) {
    if (colors.body && this.body) this.body.color.set(colors.body);
    if (colors.trim) {
      for (const m of this.trim) m.color.set(colors.trim);
      for (const p of this.parts.values()) if (this.trim.includes(p.mat)) p.base.set(colors.trim);
    }
    if (colors.light && this.lightBar) this.lightBar.emissive.set(colors.light);
  }

  /** Mark a part as the one being edited (glows softly). */
  select(button: number | null) {
    this.selected = button;
  }

  /** Show the controller's live state. */
  show(pad: Gamepad | null, accent: string) {
    const glow = new THREE.Color(accent);
    for (const [i, p] of this.parts) {
      const b = pad?.buttons[i];
      const v = b ? Math.max(b.value, b.pressed ? 1 : 0) : 0;
      const sel = this.selected === i;
      p.mat.emissive?.copy(glow);
      p.mat.emissiveIntensity = v > 0.05 ? 0.9 * v + 0.3 : sel ? 0.35 + 0.25 * Math.sin(performance.now() / 260) : 0;
      if (p.kind === 'sink') p.mesh.position.y = p.rest.y - v * 0.22;
      else p.mesh.rotation.x = v * 0.45;
    }
    const tilt = (s: THREE.Object3D | null, x: number, y: number) => {
      if (!s) return;
      s.rotation.z = -x * 0.38;
      s.rotation.x = y * 0.38;
    };
    tilt(this.sticks[0], pad?.axes[0] ?? 0, pad?.axes[1] ?? 0);
    tilt(this.sticks[1], pad?.axes[2] ?? 0, pad?.axes[3] ?? 0);
  }

  private loop = () => {
    this.frame = requestAnimationFrame(this.loop);
    if (document.visibilityState !== 'visible') return;
    this.root.rotation.y = Math.sin(performance.now() / 4000) * 0.06;
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
  };

  dispose() {
    cancelAnimationFrame(this.frame);
    this.canvas.removeEventListener('pointerdown', this.onDown);
    this.canvas.removeEventListener('pointerup', this.onUp);
    this.resize.disconnect();
    this.controls.dispose();
    this.clear();
    this.renderer.dispose();
  }
}
