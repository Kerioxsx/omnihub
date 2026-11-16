// Text shortcuts: saved text typed into the app in front when you press its
// key (it works in games and chats). The key still reaches that app, so a
// spare key — F13–F24, Pause, Scroll Lock, the numpad — works best.

import type { Snippet, SnippetSettings } from '@shared/types';
import { Copy, Plus, Trash2, Type } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Select, Switch, Textarea, TextInput } from '../../components/ui/Form';
import { toast } from '../../state/toasts';
import { KEYS } from '../counters/SpeedrunCard';

export const TEXT_KEYS: [number, string][] = [
  ...Array.from({ length: 12 }, (_, i) => [0x7c + i, `F${i + 13}`] as [number, string]),
  [0x13, 'Pause'],
  [0x91, 'Scroll Lock'],
  [0x6a, 'Numpad *'],
  [0x6b, 'Numpad +'],
  [0x6d, 'Numpad −'],
  [0x6f, 'Numpad /'],
  ...KEYS,
];

export function TextTab() {
  const [s, setS] = useState<SnippetSettings | null>(null);
  useEffect(() => void api.snippets.get().then(setS, () => undefined), []);
  if (!s) return null;
  const save = async (next: SnippetSettings) => {
    setS(next);
    try {
      setS(
        await api.snippets.save({
          ...next,
          snippets: next.snippets.filter((x) => x.text.trim()),
        }),
      );
    } catch (e) {
      toast.error('Couldn’t save the shortcuts', errorText(e));
    }
  };
  const edit = (i: number, patch: Partial<Snippet>) =>
    setS({
      ...s,
      snippets: s.snippets.map((x, k) => (k === i ? { ...x, ...patch } : x)),
    });
  const used = new Map(s.snippets.filter((x) => x.key).map((x) => [x.key, x.name]));
  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader
          icon={Type}
          title="Text shortcuts"
          subtitle="Press a key and OmniHub types the text into the app in front — a reply, your address, a party code. The key still reaches that app, so spare keys (F13–F24, Pause, Scroll Lock) work best."
          actions={
            <div className="flex items-center gap-3">
              <label className="flex items-center gap-2 text-[13px] text-dim">
                Keys on <Switch checked={s.keysOn} onChange={(v) => void save({ ...s, keysOn: v })} label="Text shortcut keys" />
              </label>
              <Button
                size="sm"
                variant="primary"
                icon={Plus}
                onClick={() =>
                  setS({
                    ...s,
                    snippets: [...s.snippets, { id: '', name: '', text: '', key: 0 }],
                  })
                }
              >
                New shortcut
              </Button>
            </div>
          }
        />
        {s.snippets.length === 0 ? (
          <div className="mt-6 py-6 text-center text-[13px] text-faint">No shortcuts yet. Add one, give it a key, and press that key in any app.</div>
        ) : (
          <div className="mt-4 space-y-3">
            {s.snippets.map((x, i) => (
              <div key={x.id || `new-${i}`} className="rounded-xl border border-line p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <TextInput value={x.name} maxLength={40} placeholder="Name (optional)" onChange={(e) => edit(i, { name: e.target.value })} onBlur={() => x.text.trim() && void save(s)} className="h-8 min-w-[160px] flex-1" inputSize="sm" aria-label="Name" />
                  <Select
                    value={x.key}
                    onChange={(e) =>
                      void save({
                        ...s,
                        snippets: s.snippets.map((y, k) => (k === i ? { ...y, key: Number(e.target.value) } : y)),
                      })
                    }
                    className="h-8 w-[140px] py-0 text-[12.5px]"
                    aria-label="Key"
                  >
                    <option value={0}>No key</option>
                    {TEXT_KEYS.map(([vk, name]) => (
                      <option key={vk} value={vk} disabled={used.has(vk) && x.key !== vk}>
                        {name}
                        {used.has(vk) && x.key !== vk ? ` (${used.get(vk)})` : ''}
                      </option>
                    ))}
                  </Select>
                  <IconButton
                    icon={Copy}
                    label="Copy the text"
                    disabled={!x.text}
                    onClick={() =>
                      void navigator.clipboard.writeText(x.text).then(
                        () => toast.success('Copied'),
                        () => undefined,
                      )
                    }
                  />
                  <IconButton
                    icon={Trash2}
                    label="Remove"
                    onClick={() =>
                      void save({
                        ...s,
                        snippets: s.snippets.filter((_, k) => k !== i),
                      })
                    }
                  />
                </div>
                <Textarea value={x.text} maxLength={2000} rows={2} placeholder="The text to type" onChange={(e) => edit(i, { text: e.target.value })} onBlur={() => x.text.trim() && void save(s)} className="mt-2 text-[13px]" aria-label="Text" />
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
