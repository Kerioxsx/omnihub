// A mini aim trainer: Flick (hit targets as they appear for 30 seconds) and
// Reaction (click the moment it turns green). Bests are kept on this PC.

import { Crosshair, Timer, X, Zap } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented } from '../../components/ui/Form';
import { useStoredState } from '../../lib/hooks';

type Mode = 'flick' | 'reaction';
const LENGTH = 30_000;

interface Target {
  x: number;
  y: number;
  r: number;
  born: number;
}

function Flick({ onDone }: { onDone: (score: { hits: number; misses: number; avg: number }) => void }) {
  const box = useRef<HTMLDivElement>(null);
  const [t, setT] = useState<Target | null>(null);
  const [left, setLeft] = useState(LENGTH);
  const stats = useRef({ hits: 0, misses: 0, times: [] as number[] });
  const start = useRef(performance.now());
  const spawn = useCallback(() => {
    const el = box.current;
    if (!el) return;
    const r = 18 + Math.random() * 14;
    setT({ x: r + Math.random() * (el.clientWidth - 2 * r), y: r + Math.random() * (el.clientHeight - 2 * r), r, born: performance.now() });
  }, []);
  useEffect(() => {
    spawn();
    const id = window.setInterval(() => {
      const l = LENGTH - (performance.now() - start.current);
      setLeft(l);
      if (l <= 0) {
        window.clearInterval(id);
        const s = stats.current;
        onDone({ hits: s.hits, misses: s.misses, avg: s.times.length ? s.times.reduce((a, b) => a + b, 0) / s.times.length : 0 });
      }
    }, 100);
    return () => window.clearInterval(id);
  }, [spawn, onDone]);
  return (
    <div
      ref={box}
      className="relative h-full w-full cursor-crosshair select-none"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) stats.current.misses++;
      }}
    >
      <div className="pointer-events-none absolute left-4 top-3 text-[13px] text-white/70">
        {Math.ceil(left / 1000)} s · {stats.current.hits} hits
      </div>
      {t && (
        <button
          type="button"
          aria-label="Target"
          onMouseDown={(e) => {
            e.stopPropagation();
            stats.current.hits++;
            stats.current.times.push(performance.now() - t.born);
            spawn();
          }}
          className="absolute rounded-full border-4 border-white bg-red-500 shadow-[0_0_20px_rgba(239,68,68,0.7)]"
          style={{ left: t.x - t.r, top: t.y - t.r, width: t.r * 2, height: t.r * 2 }}
        />
      )}
    </div>
  );
}

function Reaction({ onDone }: { onDone: (avg: number) => void }) {
  const [phase, setPhase] = useState<'wait' | 'go' | 'early'>('wait');
  const [times, setTimes] = useState<number[]>([]);
  const goAt = useRef(0);
  const timer = useRef(0);
  const arm = useCallback(() => {
    setPhase('wait');
    timer.current = window.setTimeout(() => {
      goAt.current = performance.now();
      setPhase('go');
    }, 1200 + Math.random() * 2500);
  }, []);
  useEffect(() => {
    arm();
    return () => window.clearTimeout(timer.current);
  }, [arm]);
  const click = () => {
    if (phase === 'wait') {
      window.clearTimeout(timer.current);
      setPhase('early');
      window.setTimeout(arm, 1000);
      return;
    }
    if (phase !== 'go') return;
    const next = [...times, performance.now() - goAt.current];
    setTimes(next);
    if (next.length >= 5) onDone(next.reduce((a, b) => a + b, 0) / next.length);
    else arm();
  };
  return (
    <button type="button" onMouseDown={click} className={`grid h-full w-full place-items-center text-[22px] font-semibold text-white ${phase === 'go' ? 'bg-emerald-600' : phase === 'early' ? 'bg-amber-600' : 'bg-red-700'}`}>
      {phase === 'go' ? 'Click!' : phase === 'early' ? 'Too soon' : `Wait for green… (${times.length}/5)`}
    </button>
  );
}

export function AimTrainerCard() {
  const [mode, setMode] = useState<Mode>('flick');
  const [playing, setPlaying] = useState(false);
  const [last, setLast] = useState<string | null>(null);
  const [best, setBest] = useStoredState('games.aimBest', { flick: 0, reaction: 0 });
  return (
    <Card className="p-5">
      <CardHeader icon={Crosshair} title="Aim trainer" subtitle="A quick warm-up before you play." />
      <div className="mt-4 flex flex-wrap items-center gap-3">
        <Segmented label="Mode" value={mode} onChange={setMode} options={[{ value: 'flick', label: 'Flick (30 s)', icon: Zap }, { value: 'reaction', label: 'Reaction', icon: Timer }]} />
        <Button variant="primary" onClick={() => setPlaying(true)}>
          Start
        </Button>
        <span className="text-[12.5px] text-dim">
          Best: {mode === 'flick' ? (best.flick ? `${best.flick} points` : '—') : best.reaction ? `${Math.round(best.reaction)} ms` : '—'}
        </span>
      </div>
      {last && <p className="mt-3 text-[13px] text-fg">{last}</p>}
      {playing && (
        <div className="fixed inset-0 z-[80] bg-[#0b0b12]">
          <button type="button" aria-label="Close" onClick={() => setPlaying(false)} className="absolute right-4 top-3 z-10 rounded-lg p-2 text-white/70 hover:bg-white/10">
            <X size={18} />
          </button>
          {mode === 'flick' ? (
            <Flick
              onDone={({ hits, misses, avg }) => {
                const acc = hits + misses ? hits / (hits + misses) : 0;
                const score = Math.round(hits * 100 * acc);
                setLast(`${hits} hits, ${Math.round(acc * 100)}% accuracy, ${Math.round(avg)} ms per target: ${score} points.`);
                if (score > best.flick) setBest({ ...best, flick: score });
                setPlaying(false);
              }}
            />
          ) : (
            <Reaction
              onDone={(avg) => {
                setLast(`Average reaction: ${Math.round(avg)} ms.`);
                if (!best.reaction || avg < best.reaction) setBest({ ...best, reaction: avg });
                setPlaying(false);
              }}
            />
          )}
        </div>
      )}
    </Card>
  );
}
