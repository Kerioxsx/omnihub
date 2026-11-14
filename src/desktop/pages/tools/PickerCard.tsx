// Opens the colour picker and ruler (a frozen copy of the screen), and shows
// the colours picked lately (the overlay window keeps them in localStorage).

import { Languages, Pipette } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { toast } from '../../state/toasts';
import { PICKED_KEY, readPicked } from '../overlay/PickerOverlay';

export function PickerCard() {
  const [colours, setColours] = useState<string[]>(readPicked);
  useEffect(() => {
    const on = (e: StorageEvent) => e.key === PICKED_KEY && setColours(readPicked());
    const focus = () => setColours(readPicked());
    window.addEventListener('storage', on);
    window.addEventListener('focus', focus);
    return () => {
      window.removeEventListener('storage', on);
      window.removeEventListener('focus', focus);
    };
  }, []);
  const start = (mode: 'pick' | 'translate') => void api.shots.toolBegin(mode).catch((e: unknown) => toast.error('Couldn’t freeze the screen', errorText(e)));
  return (
    <div className="mb-4 grid gap-4 lg:grid-cols-2">
      <Card className="p-5">
        <CardHeader
          icon={Pipette}
          title="Colour picker & ruler"
          subtitle="Freezes the screen: click any pixel to copy its colour, drag to measure in pixels."
          actions={
            <Button variant="primary" icon={Pipette} onClick={() => start('pick')}>
              Pick from screen
            </Button>
          }
        />
        {colours.length > 0 && (
          <div className="mt-3 flex flex-wrap gap-2">
            {colours.map((c) => (
              <button
                key={c}
                type="button"
                onClick={() =>
                  void navigator.clipboard.writeText(c).then(
                    () => toast.success(`Copied ${c}`),
                    () => undefined,
                  )
                }
                className="flex items-center gap-2 rounded-lg border border-line px-2 py-1 font-mono text-[12px] hover:bg-surface-2"
                title="Copy"
              >
                <span className="h-4 w-4 rounded border border-line" style={{ background: c }} />
                {c}
              </button>
            ))}
          </div>
        )}
      </Card>
      <Card className="p-5">
        <CardHeader
          icon={Languages}
          title="Screen translator"
          subtitle="Freezes the screen: drag over text in a game or app and it’s translated by the AI model on this PC (Ollama). Nothing is sent anywhere."
          actions={
            <Button variant="primary" icon={Languages} onClick={() => start('translate')}>
              Translate screen
            </Button>
          }
        />
      </Card>
    </div>
  );
}
