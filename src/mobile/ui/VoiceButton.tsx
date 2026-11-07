// Voice commands: the phone's own speech recognition turns what you say
// into text ("start Fortnite", "quiet mode", "clip that", "open Spotify"),
// and the PC does it. Browsers allow the microphone only over the secure
// (HTTPS) connection; without it, or where the browser has no speech
// recognition, you type the command instead.

import { LoaderCircle, Mic } from 'lucide-react';
import { useRef, useState } from 'react';
import { client } from '../client';
import { cx, errorMessage, vibrate } from '../lib/util';
import { toast } from '../state';

type Recognition = { lang: string; interimResults: boolean; maxAlternatives: number; start: () => void; stop: () => void; onresult: ((e: { results: ArrayLike<ArrayLike<{ transcript: string }>> }) => void) | null; onerror: ((e: { error: string }) => void) | null; onend: (() => void) | null };

function recognizer(): Recognition | null {
  const w = window as unknown as { SpeechRecognition?: new () => Recognition; webkitSpeechRecognition?: new () => Recognition };
  const C = w.SpeechRecognition ?? w.webkitSpeechRecognition;
  return C && window.isSecureContext ? new C() : null;
}

export function VoiceButton({ className }: { className?: string }) {
  const [state, setState] = useState<'idle' | 'listening' | 'sending'>('idle');
  const rec = useRef<Recognition | null>(null);
  const send = async (text: string) => {
    if (!text.trim()) return;
    setState('sending');
    try {
      const r = await client.voice(text);
      vibrate(20);
      toast.success(r.said, `“${text}”`);
    } catch (e) {
      vibrate([40, 60, 40]);
      toast.error(errorMessage(e));
    } finally {
      setState('idle');
    }
  };
  const start = () => {
    if (state === 'listening') {
      rec.current?.stop();
      return;
    }
    const r = recognizer();
    if (!r) {
      const t = window.prompt('Tell your PC what to do (like “start Fortnite” or “quiet mode”)');
      if (t) void send(t);
      return;
    }
    rec.current = r;
    r.lang = navigator.language || 'en-US';
    r.interimResults = false;
    r.maxAlternatives = 1;
    let heard = '';
    r.onresult = (e) => {
      heard = e.results[0]?.[0]?.transcript ?? '';
    };
    r.onerror = (e) => {
      if (e.error === 'not-allowed') toast.error('The microphone is blocked', 'Allow it for this page in the browser’s settings.');
    };
    r.onend = () => {
      rec.current = null;
      if (heard) void send(heard);
      else setState('idle');
    };
    vibrate(15);
    setState('listening');
    r.start();
  };
  return (
    <button type="button" aria-label="Voice command" onClick={start} disabled={state === 'sending'} className={cx('press relative grid h-10 w-10 place-items-center rounded-full', state === 'listening' ? 'bg-bad text-white' : 'bg-accent-soft text-accent', className)}>
      {state === 'listening' && <span className="absolute inset-0 animate-ping rounded-full bg-bad/50" />}
      {state === 'sending' ? <LoaderCircle size={19} className="spin" /> : <Mic size={19} className="relative" />}
    </button>
  );
}
