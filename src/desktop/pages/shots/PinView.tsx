// A pinned screenshot: its own small window above everything else (window
// label "pin-…", route #/pin?id=…). Drag it anywhere, resize it from its
// edges, scroll to make it see-through, Esc or × to close it.

import { X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, inTauri } from '../../api';
import { useRoute } from '../../lib/router';

async function closeWindow() {
  if (!inTauri) return;
  const { getCurrentWindow } = await import('@tauri-apps/api/window');
  void getCurrentWindow().close();
}

export function PinView() {
  const id = useRoute().params.get('id') ?? '';
  const [src, setSrc] = useState<string | null>(null);
  const [opacity, setOpacity] = useState(1);
  useEffect(() => {
    document.documentElement.classList.add('overlay-route');
    void api.shots.image(id).then(setSrc, () => void closeWindow());
    const onKey = (e: KeyboardEvent) => e.key === 'Escape' && void closeWindow();
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [id]);
  return (
    <div
      className="group relative h-screen w-screen select-none overflow-hidden bg-black"
      style={{ opacity }}
      data-tauri-drag-region
      onWheel={(e) => setOpacity((o) => Math.min(1, Math.max(0.25, o - Math.sign(e.deltaY) * 0.1)))}
      onDoubleClick={() => void closeWindow()}
      title="Drag to move · edges to resize · scroll to fade · double-click or Esc to close"
    >
      {src && <img src={src} alt="Pinned screenshot" className="pointer-events-none h-full w-full object-contain" draggable={false} />}
      <button type="button" onClick={() => void closeWindow()} aria-label="Close" className="absolute right-1.5 top-1.5 grid h-7 w-7 place-items-center rounded-full bg-black/60 text-white opacity-0 transition-opacity group-hover:opacity-100">
        <X size={15} />
      </button>
    </div>
  );
}
