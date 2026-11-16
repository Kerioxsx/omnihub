// Drop files or folders anywhere on the window to send them to a phone —
// or to whatever a page has claimed drops for (Hidden files hides them).

import { AnimatePresence, motion } from 'motion/react';
import { type LucideIcon, Smartphone } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { inTauri } from '../api';
import { useSend } from '../state/send';

export interface DropClaim {
  title: string;
  detail: string;
  icon: LucideIcon;
  onDrop: (paths: string[]) => void;
}

const SEND: DropClaim = { title: 'Drop to send to your phone', detail: "Files go to the phone's inbox; folders are zipped first.", icon: Smartphone, onDrop: (paths) => useSend.getState().openFiles(paths) };
const claims: { current: DropClaim }[] = [];
const target = () => claims[claims.length - 1]?.current ?? SEND;

/** While the calling component is mounted, files dropped on the window go to `claim`. */
export function useDropClaim(claim: DropClaim) {
  const ref = useRef(claim);
  ref.current = claim;
  useEffect(() => {
    claims.push(ref);
    return () => void claims.splice(claims.indexOf(ref), 1);
  }, []);
}

export function DropOverlay() {
  const [over, setOver] = useState<DropClaim | null>(null);

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    if (inTauri) {
      // Tauri reports native drops with real paths (HTML5 drops have none).
      void import('@tauri-apps/api/webview').then(async ({ getCurrentWebview }) => {
        const off = await getCurrentWebview().onDragDropEvent((e) => {
          const p = e.payload;
          if (p.type === 'enter' || p.type === 'over') setOver(target());
          else if (p.type === 'leave') setOver(null);
          else if (p.type === 'drop') {
            setOver(null);
            if (p.paths.length) target().onDrop(p.paths);
          }
        });
        if (alive) unlisten = off;
        else off();
      });
      return () => {
        alive = false;
        unlisten?.();
      };
    }
    // Browser (mock): use file names as stand-in paths.
    const hasFiles = (e: DragEvent) => !!e.dataTransfer && [...e.dataTransfer.types].includes('Files');
    let depth = 0;
    const enter = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      depth++;
      setOver(target());
    };
    const leave = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      depth = Math.max(0, depth - 1);
      if (!depth) setOver(null);
    };
    const overFn = (e: DragEvent) => {
      if (hasFiles(e)) e.preventDefault();
    };
    const drop = (e: DragEvent) => {
      if (!hasFiles(e)) return;
      e.preventDefault();
      depth = 0;
      setOver(null);
      const names = [...(e.dataTransfer?.files ?? [])].map((f) => `C:\\Users\\Player\\Desktop\\${f.name}`);
      if (names.length) target().onDrop(names);
    };
    window.addEventListener('dragenter', enter);
    window.addEventListener('dragleave', leave);
    window.addEventListener('dragover', overFn);
    window.addEventListener('drop', drop);
    return () => {
      window.removeEventListener('dragenter', enter);
      window.removeEventListener('dragleave', leave);
      window.removeEventListener('dragover', overFn);
      window.removeEventListener('drop', drop);
    };
  }, []);

  return (
    <AnimatePresence>
      {over && (
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="pointer-events-none fixed inset-0 z-[80] flex items-center justify-center bg-[color-mix(in_oklab,var(--bg)_70%,transparent)] backdrop-blur-sm">
          <motion.div initial={{ scale: 0.94 }} animate={{ scale: 1 }} className="flex flex-col items-center gap-3 rounded-[28px] border-2 border-dashed border-accent/60 bg-[var(--bg-elev)] px-16 py-12 text-center shadow-[0_30px_80px_-30px_var(--accent-glow)]">
            <span className="flex h-16 w-16 items-center justify-center rounded-[20px] bg-accent-soft text-accent">
              <over.icon size={30} aria-hidden />
            </span>
            <div className="font-display text-[19px] font-semibold text-fg">{over.title}</div>
            <div className="text-[13px] text-dim">{over.detail}</div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
