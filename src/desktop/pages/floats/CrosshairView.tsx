// The crosshair window: transparent, click-through, over everything, the
// crosshair in the middle of the main screen. Follows the settings live.

import { useEffect, useState } from 'react';
import { CrosshairShape } from './CrosshairShape';
import { KEY, readLook } from './crosshair';

export function CrosshairView() {
  const [look, setLook] = useState(readLook);
  useEffect(() => {
    const on = (e: StorageEvent) => e.key === KEY && setLook(readLook());
    window.addEventListener('storage', on);
    return () => window.removeEventListener('storage', on);
  }, []);
  return (
    <div className="fixed inset-0 grid place-items-center" style={{ background: 'transparent' }}>
      <CrosshairShape look={look} box={160} />
    </div>
  );
}
