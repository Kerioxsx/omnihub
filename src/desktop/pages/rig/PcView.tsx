// The 3D PC (three.js, loaded only when this page opens).

import type { LightProfile, RigSettings, Specs } from '@shared/types';
import { useEffect, useRef, useState } from 'react';
import { LevelFollower, useAudioFeed } from '../../lib/aurora/audio';
import type { PartId, PcScene } from '../../lib/rig/scene3d';
import { Spinner } from '../../components/ui/Card';

export function PcView({ specs, model, profile, rpm, temp, media, highlight, className }: { specs: Specs | null; model: RigSettings['model'] | null; profile: LightProfile | null; rpm: number | null; temp: number | null; media: { url: string; video: boolean } | null; highlight: PartId | null; className?: string }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [scene, setScene] = useState<PcScene | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const tempRef = useRef(temp);
  tempRef.current = temp;
  useAudioFeed(profile?.effect === 'music', 'rig-preview');

  useEffect(() => {
    let alive = true;
    let made: PcScene | null = null;
    void import('../../lib/rig/scene3d').then(
      ({ PcScene }) => {
        if (!alive || !canvas.current) return;
        try {
          made = new PcScene(canvas.current);
          const follow = new LevelFollower();
          let last = performance.now();
          made.setInputs(() => {
            const now = performance.now();
            const v = follow.step(Math.min(0.1, (now - last) / 1000));
            last = now;
            return { level: v.level, beat: v.beat, temp: tempRef.current };
          });
          setScene(made);
        } catch (e) {
          setFailed(e instanceof Error ? e.message : String(e));
        }
      },
      (e: unknown) => setFailed(String(e)),
    );
    return () => {
      alive = false;
      made?.dispose();
    };
  }, []);

  const modelKey = JSON.stringify(model);
  const specsKey = specs ? `${specs.boardSize}|${specs.cooler}|${specs.radiatorMm}|${specs.ram.length}|${specs.gpus[0] ?? ''}` : '';
  useEffect(() => {
    if (scene && model) scene.build(specs, model);
  }, [scene, modelKey, specsKey]);
  useEffect(() => scene?.setLighting(profile), [scene, profile]);
  useEffect(() => scene?.setRpm(rpm), [scene, rpm]);
  useEffect(() => scene?.setMedia(media), [scene, media]);
  useEffect(() => scene?.setHighlight(highlight), [scene, highlight]);

  return (
    <div className={className}>
      <canvas ref={canvas} className="block h-full w-full touch-none" aria-label="Your PC in 3D. Drag to turn it, scroll to zoom." />
      {!scene && !failed && (
        <div className="absolute inset-0 grid place-items-center text-dim">
          <Spinner size={22} />
        </div>
      )}
      {failed && <div className="absolute inset-0 grid place-items-center p-6 text-center text-[13px] text-faint">The 3D view needs WebGL, which isn’t available here ({failed}).</div>}
    </div>
  );
}
