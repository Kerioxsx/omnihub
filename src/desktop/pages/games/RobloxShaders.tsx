// Roblox shaders: Roblox's anti-tamper now stops it from starting with
// ReShade in its folder, so this only says so (the backend takes any copy
// OmniHub put there back out).

import type { RobloxRenderer, RobloxShaders as Shaders, ShaderStatus } from '@shared/types';
import { AlertTriangle, Sparkles } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api } from '../../api';
import { Callout } from '../../components/ui/States';

export function RobloxShaders({ value, onChange }: { value: Shaders; onChange: (s: Shaders) => void; renderer: RobloxRenderer; found: boolean }) {
  const [status, setStatus] = useState<ShaderStatus | null>(null);
  // Roblox no longer starts with ReShade in its folder: OmniHub keeps it
  // out (and has taken out any it put there before).
  useEffect(() => {
    void api.games.robloxShaders().then(setStatus, () => undefined);
    if (value.enabled) onChange({ ...value, enabled: false });
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <div className="space-y-3 border-t border-line px-5 py-4">
      <div className="flex items-center gap-2 text-[13.5px] font-medium text-fg">
        <Sparkles className="h-4 w-4 text-accent" aria-hidden /> Shaders
      </div>
      <Callout tone="warn" icon={AlertTriangle} className="text-[12.5px]">
        Roblox now blocks shader tools: with ReShade in its folder Roblox stops at “Failed to load library: d3d11.dll” (Bloxshade is down for the same reason). OmniHub took its shaders out so Roblox starts, and will bring them back here if Roblox allows them again.
      </Callout>
      {status && status.foreign.length > 0 && (
        <Callout tone="warn" icon={AlertTriangle} className="text-[12.5px]">
          Another shader tool’s ReShade is still in Roblox’s folder and can stop Roblox from starting: uninstall it, or delete <code>dxgi.dll</code> from {status.foreign[0]}.
        </Callout>
      )}
    </div>
  );
}
