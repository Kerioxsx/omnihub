// The phone as a controller: a pad whose buttons hold keys on the PC
// (RetroArch's keyboard layout, which most emulators can use). Keys are let
// go when you lift your finger, leave this page, or the phone goes quiet.

import { useEffect, useRef } from 'react';
import { client } from '../client';
import { vibrate } from '../lib/util';
import { toast } from '../state';
import { SubHeader } from './More';

function Btn({ id, label, className }: { id: string; label: string; className?: string }) {
  const down = useRef(false);
  const send = (d: boolean) => {
    if (down.current === d) return;
    down.current = d;
    if (d) vibrate(8);
    void client.pad(id, d).catch((e: unknown) => d && toast.error(e instanceof Error ? e.message : 'The PC didn’t take it'));
  };
  return (
    <button
      type="button"
      onPointerDown={(e) => {
        (e.target as HTMLElement).setPointerCapture(e.pointerId);
        send(true);
      }}
      onPointerUp={() => send(false)}
      onPointerCancel={() => send(false)}
      onContextMenu={(e) => e.preventDefault()}
      className={`touch-none select-none rounded-full border border-white/15 bg-white/10 font-semibold text-white active:bg-accent/60 ${className ?? ''}`}
    >
      {label}
    </button>
  );
}

export function ControllerPage({ active, onBack }: { active: boolean; onBack: () => void }) {
  useEffect(() => {
    if (!active) return;
    // While a button is held the PC hears from the phone every second.
    const alive = window.setInterval(() => void client.pad('alive', true).catch(() => undefined), 1000);
    const release = () => void client.pad('release', false).catch(() => undefined);
    document.addEventListener('visibilitychange', release);
    return () => {
      window.clearInterval(alive);
      document.removeEventListener('visibilitychange', release);
      release();
    };
  }, [active]);
  return (
    <div className="flex min-h-full flex-col pb-6">
      <SubHeader title="Controller" onBack={onBack} />
      <p className="px-4 pb-3 text-[12.5px] text-white/60">Holds keys on the PC: arrows, X (A), Z (B), S (X), A (Y), Q / W (L / R), Enter (Start), Right Shift (Select) — RetroArch’s defaults. Turn the phone sideways.</p>
      <div className="mx-4 flex flex-1 flex-col justify-center gap-6 rounded-3xl border border-white/10 bg-black/30 p-4">
        <div className="flex justify-between">
          <Btn id="l" label="L" className="h-12 w-24" />
          <Btn id="r" label="R" className="h-12 w-24" />
        </div>
        <div className="flex items-center justify-between">
          <div className="grid grid-cols-3 grid-rows-3 gap-1">
            <span />
            <Btn id="up" label="▲" className="h-14 w-14 rounded-xl" />
            <span />
            <Btn id="left" label="◀" className="h-14 w-14 rounded-xl" />
            <span />
            <Btn id="right" label="▶" className="h-14 w-14 rounded-xl" />
            <span />
            <Btn id="down" label="▼" className="h-14 w-14 rounded-xl" />
            <span />
          </div>
          <div className="grid grid-cols-3 grid-rows-3 gap-1">
            <span />
            <Btn id="y" label="Y" className="h-14 w-14" />
            <span />
            <Btn id="x" label="X" className="h-14 w-14" />
            <span />
            <Btn id="a" label="A" className="h-14 w-14" />
            <span />
            <Btn id="b" label="B" className="h-14 w-14" />
            <span />
          </div>
        </div>
        <div className="flex justify-center gap-4">
          <Btn id="select" label="Select" className="h-10 w-24 text-[13px]" />
          <Btn id="start" label="Start" className="h-10 w-24 text-[13px]" />
        </div>
      </div>
    </div>
  );
}
