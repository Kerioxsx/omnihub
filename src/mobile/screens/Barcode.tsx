// Scan a barcode or QR code with the phone and type it on the PC where the
// cursor is (or copy it there, or open its link). Uses the browser's own
// barcode reader.

import { Camera, ClipboardCopy, ExternalLink, Keyboard, ScanLine } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { client } from '../client';
import { errorMessage, vibrate } from '../lib/util';
import { toast, useApp } from '../state';
import { Button, Empty, Switch } from '../ui/common';
import { SubHeader } from './More';

interface Detector {
  detect(src: CanvasImageSource | ImageBitmap): Promise<{ rawValue: string; format: string }[]>;
}
declare global {
  interface Window {
    BarcodeDetector?: { new (opts?: { formats?: string[] }): Detector; getSupportedFormats?: () => Promise<string[]> };
  }
}

export function BarcodePage({ active, onBack }: { active: boolean; onBack: () => void }) {
  const f = useApp((s) => s.info?.features);
  const video = useRef<HTMLVideoElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const detector = useRef<Detector | null>(null);
  const [live, setLive] = useState(false);
  const [code, setCode] = useState<{ value: string; format: string } | null>(null);
  const [auto, setAuto] = useState(false);
  const [enter, setEnter] = useState(true);
  const [text, setText] = useState('');
  const last = useRef({ value: '', at: 0 });
  const supported = typeof window !== 'undefined' && !!window.BarcodeDetector;
  const camera = !!navigator.mediaDevices?.getUserMedia;

  const send = async (value: string, how: 'type' | 'copy' | 'open') => {
    try {
      if (how === 'type') await client.typeText(value, enter);
      else if (how === 'copy') await client.sendClipboard(value);
      else await client.openLink(value);
      toast.success(how === 'type' ? 'Typed on the PC' : how === 'copy' ? 'On the PC clipboard' : 'Opened on the PC', value.slice(0, 60));
    } catch (e) {
      toast.error('Couldn’t send it', errorMessage(e));
    }
  };

  const found = (value: string, format: string) => {
    const now = Date.now();
    if (value === last.current.value && now - last.current.at < 2500) return;
    last.current = { value, at: now };
    vibrate(30);
    setCode({ value, format });
    if (auto) void send(value, 'type');
  };

  // The live camera, read a few times a second.
  useEffect(() => {
    if (!live || !active || !supported) return;
    let stream: MediaStream | null = null;
    let timer = 0;
    let stop = false;
    detector.current ??= new window.BarcodeDetector!();
    navigator.mediaDevices
      .getUserMedia({ video: { facingMode: 'environment', width: { ideal: 1920 } }, audio: false })
      .then((s) => {
        if (stop) return s.getTracks().forEach((t) => t.stop());
        stream = s;
        const v = video.current;
        if (!v) return;
        v.srcObject = s;
        void v.play();
        const tick = async () => {
          if (stop) return;
          if (v.readyState >= 2) {
            try {
              const r = await detector.current!.detect(v);
              if (r[0]?.rawValue) found(r[0].rawValue, r[0].format);
            } catch {
              /* a frame that couldn't be read */
            }
          }
          timer = window.setTimeout(() => void tick(), 250);
        };
        void tick();
      })
      .catch((e: unknown) => {
        toast.error('No camera', errorMessage(e));
        setLive(false);
      });
    return () => {
      stop = true;
      clearTimeout(timer);
      stream?.getTracks().forEach((t) => t.stop());
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [live, active, supported]);

  const fromPhoto = async (file: File | undefined) => {
    if (!file || !supported) return;
    try {
      detector.current ??= new window.BarcodeDetector!();
      const r = await detector.current.detect(await createImageBitmap(file));
      if (r[0]?.rawValue) found(r[0].rawValue, r[0].format);
      else toast.error('No code found', 'Get closer, with the code sharp and well lit.');
    } catch (e) {
      toast.error('Couldn’t read it', errorMessage(e));
    }
  };

  const isLink = !!code && /^https?:\/\//i.test(code.value);
  return (
    <div className="scroller h-full">
      <SubHeader title="Barcode to PC" subtitle="Scan, then type it where the PC’s cursor is" onBack={onBack} />
      <input ref={input} type="file" accept="image/*" capture="environment" className="hidden" onChange={(e) => void fromPhoto(e.target.files?.[0]).finally(() => (e.target.value = ''))} />
      <div className="px-safe space-y-4 pb-tabbar">
        {!f?.control && (
          <div className="rounded-2xl border border-warn/30 bg-warn/10 p-3 text-[13px] text-warn">Typing on the PC needs “Let phones use the mouse and keyboard” on in OmniHub on the PC (Phone → Permissions). Copy works without it.</div>
        )}
        {supported ? (
          <>
            {live ? (
              <div className="relative overflow-hidden rounded-2xl bg-black">
                <video ref={video} className="aspect-[3/4] w-full object-cover" playsInline muted />
                <div className="pointer-events-none absolute inset-x-8 top-1/2 h-0.5 -translate-y-1/2 bg-rose-500/80 shadow-[0_0_12px_rgba(244,63,94,.8)]" />
              </div>
            ) : (
              <Empty icon={<ScanLine size={28} />} title="Point the camera at a code" body="Barcodes on products, QR codes, ISBNs. Handy for inventory sheets: turn on “Type each one” and scan away." />
            )}
            <div className="grid grid-cols-2 gap-2.5">
              {camera && (
                <Button size="lg" variant={live ? 'ghost' : 'primary'} icon={<Camera size={20} />} onClick={() => setLive(!live)}>
                  {live ? 'Stop camera' : 'Live camera'}
                </Button>
              )}
              <Button size="lg" variant="ghost" className={camera ? '' : 'col-span-2'} icon={<Camera size={20} />} onClick={() => input.current?.click()}>
                Take a photo
              </Button>
            </div>
          </>
        ) : (
          <Empty icon={<ScanLine size={28} />} title="This browser can’t read barcodes" body="Chrome on Android can. On iPhone, scan with the Camera app, copy the text, and paste it below." />
        )}
        <div className="card divide-y divide-line">
          <label className="flex items-center justify-between gap-3 px-4 py-3 text-[15px]">
            Type each one on the PC as it’s scanned
            <Switch on={auto} onChange={setAuto} label="Type automatically" />
          </label>
          <label className="flex items-center justify-between gap-3 px-4 py-3 text-[15px]">
            Press Enter after it
            <Switch on={enter} onChange={setEnter} label="Press Enter" />
          </label>
        </div>
        {code && (
          <div className="card space-y-3 p-4">
            <div className="text-[12px] uppercase tracking-wider text-faint">{code.format.replace(/_/g, ' ')}</div>
            <div className="break-all font-mono text-[16px]">{code.value}</div>
            <div className="grid grid-cols-2 gap-2">
              <Button icon={<Keyboard size={18} />} onClick={() => void send(code.value, 'type')}>
                Type on PC
              </Button>
              <Button variant="ghost" icon={<ClipboardCopy size={18} />} onClick={() => void send(code.value, 'copy')}>
                Copy on PC
              </Button>
              {isLink && (
                <Button variant="ghost" className="col-span-2" icon={<ExternalLink size={18} />} onClick={() => void send(code.value, 'open')}>
                  Open on PC
                </Button>
              )}
            </div>
          </div>
        )}
        <div className="card space-y-2 p-3">
          <textarea value={text} onChange={(e) => setText(e.target.value)} rows={3} placeholder="Or type or paste text to type on the PC" className="w-full resize-none rounded-xl bg-surface-2 p-3 text-[15px] outline-none" />
          <Button className="w-full" icon={<Keyboard size={18} />} disabled={!text.trim()} onClick={() => void send(text, 'type').then(() => setText(''))}>
            Type it on the PC
          </Button>
        </div>
      </div>
    </div>
  );
}
