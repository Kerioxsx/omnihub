import type { Note, NoteKind } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { Bot, LayoutTemplate, Lightbulb, NotebookPen, Plus } from 'lucide-react';
import type { VaultStatus } from '@shared/types';
import { useCallback, useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Drawer, Modal } from '../../components/ui/Overlay';
import { EmptyState } from '../../components/ui/States';
import { useDebounced, useEvent, useMediaQuery, useStoredState } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { confirm } from '../../state/dialogs';
import { useLive } from '../../state/live';
import { toast } from '../../state/toasts';
import { LockScreen } from '../vault/LockScreen';
import { ClaudePanel } from './ClaudePanel';
import { type Draft, NoteEditor } from './NoteEditor';
import { NoteList } from './NoteList';
import { fillTemplate } from './templates';
import { type TemplateChoice, TemplatePicker } from './TemplatePicker';

const sortNotes = (a: Note, b: Note) => Number(b.pinned) - Number(a.pinned) || b.updated - a.updated;
const toDraft = (n: Note, body = n.body): Draft => ({ title: n.title, body, tags: [...n.tags], pinned: n.pinned, color: n.color });

export function NotesPage() {
  const route = useRoute();
  const [kind, setKind] = useState<NoteKind>(route.params.get('tab') === 'idea' || route.params.get('new') === 'idea' ? 'idea' : 'note');
  const [notes, setNotes] = useState<Note[]>([]);
  const [loading, setLoading] = useState(true);
  const [query, setQuery] = useState('');
  const dq = useDebounced(query, 200);
  const [tag, setTag] = useState<string | null>(null);
  const [tags, setTags] = useState<[string, number][]>([]);
  const [current, setCurrent] = useState<Note | null>(null);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [saving, setSaving] = useState<'idle' | 'pending' | 'saving' | 'saved'>('idle');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [panel, setPanel] = useStoredState('omnihub.notes.claudePanel', true);
  const wide = useMediaQuery('(min-width: 1280px)');
  const [panelDrawer, setPanelDrawer] = useState(false);
  const [templatesOpen, setTemplatesOpen] = useState(false);
  /** The current note is locked and the vault is too: the unlock form shows instead of the text. */
  const [sealed, setSealed] = useState(false);
  /** Locking waits for the vault to be unlocked. */
  const [lockAsk, setLockAsk] = useState(false);
  const vault = useLive((s) => s.vault);
  const lastTouch = useRef(0);
  const dirty = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latest = useRef<{ note: Note | null; draft: Draft | null }>({ note: null, draft: null });
  latest.current = { note: current, draft };
  const handledParams = useRef('');

  const loadTags = useCallback(() => void api.notes.tags().then(setTags, () => undefined), []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.notes.list({ kind, query: dq || undefined, tag });
      setNotes(list);
    } catch (e) {
      toast.error('Could not load notes', errorText(e));
    } finally {
      setLoading(false);
    }
  }, [kind, dq, tag]);

  useEffect(() => {
    void load();
  }, [load]);
  useEffect(loadTags, [loadTags]);

  const save = useCallback(async () => {
    const { note, draft: d } = latest.current;
    if (!note || !d || !dirty.current) return;
    dirty.current = false;
    if (timer.current) clearTimeout(timer.current);
    setSaving('saving');
    try {
      const saved = await api.notes.save({ id: note.id, kind: note.kind, title: d.title, body: d.body, tags: d.tags, pinned: d.pinned, color: d.color, locked: note.locked });
      setCurrent((c) => (c?.id === saved.id ? saved : c));
      setNotes((list) => list.map((n) => (n.id === saved.id ? saved : n)).sort(sortNotes));
      setSaving('saved');
      setSaveError(null);
      loadTags();
    } catch (e) {
      dirty.current = true;
      setSaveError(errorText(e));
      setSaving('idle');
    }
  }, [loadTags]);

  // Flush pending edits when leaving the page.
  useEffect(() => () => void save(), [save]);

  const select = useCallback(
    async (id: string) => {
      if (latest.current.note?.id === id) return;
      await save();
      const n = notes.find((x) => x.id === id) ?? (await api.notes.get(id).catch(() => null));
      if (!n) return;
      // A locked note's text comes from the vault, when it's unlocked.
      const body = n.locked ? await api.notes.readLocked(id).catch(() => null) : n.body;
      setSealed(body == null);
      setCurrent(n);
      setDraft(toDraft(n, body ?? ''));
      setSaving('idle');
      setSaveError(null);
    },
    [notes, save],
  );

  const onDraft = (d: Draft) => {
    // Writing in a locked note keeps the vault from locking under it.
    if (latest.current.note?.locked && Date.now() - lastTouch.current > 15000) {
      lastTouch.current = Date.now();
      void api.vault.touch().catch(() => undefined);
    }
    setDraft(d);
    dirty.current = true;
    setSaving('pending');
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => void save(), 650);
  };

  const create = useCallback(
    async (k: NoteKind, from?: TemplateChoice) => {
      await save();
      try {
        const n = await api.notes.save({ kind: k, title: from ? fillTemplate(from.title) : '', body: from ? fillTemplate(from.body) : '', tags: from?.tags ?? [], pinned: false, color: null });
        setKind(k);
        setQuery('');
        setTag(null);
        setNotes((list) => (k === kind ? [n, ...list].sort(sortNotes) : list));
        setCurrent(n);
        setDraft(toDraft(n));
        setSealed(false);
        setSaving('idle');
      } catch (e) {
        toast.error('Could not create', errorText(e));
      }
    },
    [kind, save],
  );

  /** Keep a copy of the current note as a template (tagged #template). */
  const saveAsTemplate = async () => {
    if (!current || !draft) return;
    await save();
    try {
      await api.notes.save({ kind: current.kind, title: `${draft.title || 'Untitled'} (template)`, body: draft.body, tags: [...new Set([...draft.tags, 'template'])], pinned: false, color: draft.color });
      toast.success('Saved as a template', 'Find it under Templates.');
    } catch (e) {
      toast.error('Could not save the template', errorText(e));
    }
  };

  // Select the first note when nothing is selected (or the selection left this tab).
  useEffect(() => {
    if (loading) return;
    if (!current || current.kind !== kind) {
      if (notes[0]) void select(notes[0].id);
      else {
        setCurrent(null);
        setDraft(null);
      }
    }
  }, [loading, notes, kind]);

  // ?new=idea, ?id=… from the palette, Home or the phone.
  useEffect(() => {
    const key = route.params.toString();
    if (!key || key === handledParams.current) return;
    handledParams.current = key;
    const n = route.params.get('new');
    const id = route.params.get('id');
    if (n === 'note' || n === 'idea') void create(n);
    else if (id) void select(id);
    navigate('notes');
  }, [route.params, create, select]);

  useEvent<{ id: string; deleted?: boolean }>('notes:changed', (p) => {
    if (p.id === latest.current.note?.id && !p.deleted) return;
    void load();
    loadTags();
  });
  useEvent<{ id: string; path: string }>('notes:exported', (p) => {
    if (p.id === latest.current.note?.id) void api.notes.get(p.id).then((n) => setCurrent(n), () => undefined);
  });

  // The vault locked: a locked note's text goes away until it's unlocked again.
  useEvent('vault:locked', () => {
    if (!latest.current.note?.locked) return;
    dirty.current = false;
    setSealed(true);
    setDraft((d) => (d ? { ...d, body: '' } : d));
  });
  useEvent('vault:unlocked', () => {
    const n = latest.current.note;
    if (n?.locked) {
      void api.notes.readLocked(n.id).then((body) => {
        if (latest.current.note?.id !== n.id) return;
        setDraft((d) => (d ? { ...d, body } : d));
        setSealed(false);
      }, () => undefined);
    }
    if (lockAsk) {
      setLockAsk(false);
      void setLocked(true);
    }
  });

  const setLocked = async (locked: boolean) => {
    const note = latest.current.note;
    if (!note) return;
    await save();
    try {
      const n = await api.notes.setLocked(note.id, locked);
      setCurrent(n);
      setNotes((list) => list.map((x) => (x.id === n.id ? n : x)));
      if (locked) toast.success('Note locked', 'Only your vault password (or Windows Hello) opens it now. The title stays visible.');
      else toast.success('Lock taken off');
    } catch (e) {
      toast.error(locked ? 'Couldn’t lock the note' : 'Couldn’t take the lock off', errorText(e));
    }
  };

  const toggleLock = async () => {
    const note = latest.current.note;
    if (!note) return;
    if (note.locked) return void setLocked(false);
    const st: VaultStatus | null = await api.vault.status().catch(() => null);
    if (!st?.exists) {
      const go = await confirm({ title: 'Set up the vault first', description: 'Locked notes use your vault password (the same one as your passwords and hidden files). Create it in Vault, then lock this note.', confirmLabel: 'Open Vault' });
      if (go) navigate('vault');
      return;
    }
    if (!st.unlocked) {
      await useLive.getState().refreshVault();
      setLockAsk(true);
      return;
    }
    void setLocked(true);
  };

  const remove = async () => {
    if (!current) return;
    const ok = await confirm({ title: `Delete “${draft?.title || 'Untitled'}”?`, description: current.exportedPath ? 'The note is removed from OmniHub. The file already sent to Claude stays in the folder.' : 'This cannot be undone.', tone: 'danger', confirmLabel: 'Delete' });
    if (!ok) return;
    try {
      dirty.current = false;
      await api.notes.delete(current.id);
      const rest = notes.filter((n) => n.id !== current.id);
      setNotes(rest);
      setCurrent(null);
      setDraft(null);
      if (rest[0]) void select(rest[0].id);
      toast.success('Deleted');
    } catch (e) {
      toast.error('Could not delete', errorText(e));
    }
  };

  const isIdea = kind === 'idea';
  const showPanel = isIdea && panel && wide;

  return (
    <Page
      title="Notes"
      subtitle={isIdea ? 'Ideas you hand to Claude as Markdown files in a folder it can read.' : 'Quick Markdown notes, synced with the phone companion.'}
      scroll={false}
      actions={
        <>
          {isIdea && !showPanel && (
            <Button icon={Bot} onClick={() => (wide ? setPanel(true) : setPanelDrawer(true))}>
              Claude folder
            </Button>
          )}
          <Button icon={LayoutTemplate} variant="ghost" onClick={() => setTemplatesOpen(true)}>
            Templates
          </Button>
          <Button icon={isIdea ? Plus : Lightbulb} variant={isIdea ? 'secondary' : 'ghost'} onClick={() => void create(isIdea ? 'note' : 'idea')}>
            {isIdea ? 'New note' : 'New idea'}
          </Button>
          <Button variant="primary" icon={isIdea ? Lightbulb : Plus} onClick={() => void create(kind)}>
            {isIdea ? 'New idea' : 'New note'}
          </Button>
        </>
      }
    >
      <div className={showPanel ? 'grid min-h-0 flex-1 grid-cols-[290px_minmax(0,1fr)_300px] gap-3' : 'grid min-h-0 flex-1 grid-cols-[260px_minmax(0,1fr)] gap-3 xl:grid-cols-[290px_minmax(0,1fr)]'}>
        <NoteList
          kind={kind}
          onKind={(k) => {
            void save();
            setKind(k);
            setTag(null);
          }}
          notes={notes}
          loading={loading}
          selected={current?.id ?? null}
          onSelect={(id) => void select(id)}
          query={query}
          onQuery={setQuery}
          tags={tags}
          tag={tag}
          onTag={setTag}
        />
        <AnimatePresence mode="wait">
          {current && draft && sealed && vault ? (
            <motion.div key={`${current.id}-sealed`} className="card flex min-h-0 flex-col" initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
              <LockScreen status={vault} compact title={current.title || 'Locked note'} subtitle="This note is locked. Unlock with your vault password to read and edit it." />
            </motion.div>
          ) : current && draft && !sealed ? (
            <motion.div key={current.id} className="flex min-h-0 flex-col" initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: 0.16 }}>
              <NoteEditor
                onToggleLock={() => void toggleLock()}
                note={current}
                draft={draft}
                onDraft={onDraft}
                saving={saving}
                saveError={saveError}
                onDelete={() => void remove()}
                onExported={(n) => {
                  setCurrent(n);
                  setNotes((list) => list.map((x) => (x.id === n.id ? n : x)));
                }}
                onSaveTemplate={() => void saveAsTemplate()}
              />
            </motion.div>
          ) : (
            <motion.div key="empty" className="card flex items-center justify-center" initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
              <EmptyState icon={isIdea ? Lightbulb : NotebookPen} title={isIdea ? 'Capture an idea for Claude' : 'Pick or create a note'} description={isIdea ? 'Write it in Markdown, then “Send to Claude” drops it into your Claude folder as a .md file.' : 'Notes are Markdown with live preview, tags and colours.'} action={<Button variant="primary" icon={Plus} onClick={() => void create(kind)}>{isIdea ? 'New idea' : 'New note'}</Button>} />
            </motion.div>
          )}
        </AnimatePresence>
        {showPanel && <ClaudePanel onClose={() => setPanel(false)} />}
      </div>
      <Drawer open={panelDrawer && !wide} onClose={() => setPanelDrawer(false)} label="Claude folder" width={380}>
        <ClaudePanel onClose={() => setPanelDrawer(false)} className="h-full rounded-none border-0" />
      </Drawer>
      <Modal open={lockAsk && !!vault} onClose={() => setLockAsk(false)} title="Unlock the vault to lock this note" size="sm" bare>
        {vault && <LockScreen status={vault} compact title="Unlock the vault" subtitle="Locked notes are sealed with your vault key." />}
      </Modal>
      <TemplatePicker
        open={templatesOpen}
        onClose={() => setTemplatesOpen(false)}
        onPick={(t) => {
          setTemplatesOpen(false);
          void create(t.kind, t);
        }}
      />
    </Page>
  );
}
