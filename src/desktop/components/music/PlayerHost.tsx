// The full-screen player, above everything else when open.

import { AnimatePresence, motion } from 'motion/react';
import { createPortal } from 'react-dom';
import { usePlayer } from '../../lib/nowPlaying';
import { FullPlayer } from './FullPlayer';

export function PlayerHost() {
  const { open, hide } = usePlayer();
  return createPortal(
    <AnimatePresence>
      {open && (
        <motion.div key="player" role="dialog" aria-modal="true" aria-label="Music player" className="fixed inset-0 z-[200]" initial={{ opacity: 0, scale: 1.02 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 1.02 }} transition={{ duration: 0.35, ease: 'easeOut' }}>
          <FullPlayer mode="fullscreen" onClose={hide} />
        </motion.div>
      )}
    </AnimatePresence>,
    document.body,
  );
}
