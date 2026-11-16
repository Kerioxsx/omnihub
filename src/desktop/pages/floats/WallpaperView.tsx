// The live wallpaper: your OmniHub theme's animated background, behind the
// desktop icons.

import { useEffect } from 'react';
import { Backdrop } from '../../components/Backdrop';
import { ThemeController } from '../../components/GlobalEvents';
import { useSettings } from '../../state/settings';

export function WallpaperView() {
  useEffect(() => {
    void useSettings.getState().load();
  }, []);
  return (
    <div className="fixed inset-0 bg-bg">
      <ThemeController />
      <div className="app-backdrop" />
      <Backdrop />
    </div>
  );
}
