// Karaoke Studio's first question: your first language (and accent), so the
// pronunciation is written the way you read. Asked once, changeable any time.

import { Check, Languages } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Button } from '../../../components/ui/Button';
import { SearchInput } from '../../../components/ui/Form';
import { Modal } from '../../../components/ui/Overlay';
import { cx } from '../../../lib/cx';
import { languageOf, searchLanguages, suggestedLanguage } from '../../../lib/karaoke/languages';

export function LanguageSetup({ open, current, first, onSave, onClose }: { open: boolean; current: string | null; first: boolean; onSave: (code: string) => Promise<void>; onClose: () => void }) {
  const [q, setQ] = useState('');
  const [pick, setPick] = useState<string | null>(current ?? suggestedLanguage()?.code ?? null);
  const [busy, setBusy] = useState(false);
  const list = useMemo(() => searchLanguages(q), [q]);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    setQ('');
    setPick(current ?? suggestedLanguage()?.code ?? null);
  }, [open, current]);
  // Keep the chosen one in view when the list opens.
  useEffect(() => {
    if (open && !q) listRef.current?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'center' });
  }, [open, q]);

  const save = async () => {
    if (!pick) return;
    setBusy(true);
    try {
      await onSave(pick);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      icon={Languages}
      title={first ? 'Welcome to Karaoke Studio' : 'Your language'}
      description="What’s your first language? Pronunciation is written the way you read it. It doesn’t limit which songs you can sing."
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            {first ? 'Later' : 'Cancel'}
          </Button>
          <Button variant="primary" icon={Check} loading={busy} disabled={!pick} onClick={() => void save()}>
            {pick ? `Use ${languageOf(pick)?.name ?? pick}` : 'Pick a language'}
          </Button>
        </>
      }
    >
      <SearchInput value={q} onChange={setQ} placeholder="Search: Spanish, Português, हिन्दी, 日本語…" autoFocus aria-label="Search languages" />
      <div ref={listRef} role="listbox" aria-label="Languages" className="mt-3 max-h-[340px] overflow-y-auto rounded-xl border border-line">
        {list.length === 0 && <div className="p-4 text-[13px] text-faint">No language matches “{q}”.</div>}
        {list.map((l) => {
          const on = l.code === pick;
          return (
            <button
              key={l.code}
              type="button"
              role="option"
              aria-selected={on}
              onClick={() => setPick(l.code)}
              onDoubleClick={() => {
                setPick(l.code);
                void onSave(l.code);
              }}
              className={cx('flex w-full items-center justify-between gap-3 border-b border-line px-3.5 py-2.5 text-left last:border-b-0', on ? 'bg-accent-soft text-fg' : 'hover:bg-surface-2')}
            >
              <span className="min-w-0">
                <span className="block text-[13.5px] font-medium">{l.name}</span>
                <span className="block truncate text-[12px] text-faint" lang={l.code}>
                  {l.native}
                </span>
              </span>
              {on && <Check size={16} className="shrink-0 text-accent" aria-hidden />}
            </button>
          );
        })}
      </div>
    </Modal>
  );
}
