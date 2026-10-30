// Clips & Capri: instant replay ("Capri, clip that"), voice commands, and
// Capri reacting to your games. Everything is off until switched on, and
// runs only while a game does.

import { formatBytes, formatRelative } from '@shared/format';
import type { PlayClip, PlaySettings, PlayStatus } from '@shared/types';
import { Circle, Clapperboard, Download, ExternalLink, FolderOpen, Mic, MicOff, Plus, Sparkles, Square, Trash, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { Field, Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Callout, EmptyState } from '../../components/ui/States';
import { useEvent, useInterval } from '../../lib/hooks';
import { confirm } from '../../state/dialogs';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { CapriToggle } from '../chats/CapriToggle';

const EXAMPLES = ['Capri, clip that', 'Capri, record the last ten minutes', 'Capri, save the last thirty seconds', 'Capri, start recording', 'Capri, stop recording', 'Capri, call me boss', 'Capri, what’s my FPS', 'Capri, take a screenshot'];
const BUILT_IN_NAMES = ['boss', 'champ', 'chief', 'captain', 'king', 'queen', 'legend', 'daddy', 'mommy', 'bro', 'dude', 'partner', 'commander', 'master chief', 'goat'];

function Row({ title, hint, children }: { title: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-2.5">
      <div className="min-w-0">
        <div className="text-[13.5px] font-medium text-fg">{title}</div>
        {hint && <div className="text-[12px] text-faint">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

const mmss = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;

export function ClipsTab() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [st, setSt] = useState<PlayStatus | null>(null);
  const [clips, setClips] = useState<PlayClip[] | null>(null);
  const [install, setInstall] = useState<{ done: number; total: number } | null>(null);
  const [newName, setNewName] = useState('');
  const [saving, setSaving] = useState(false);

  const load = useCallback(() => {
    void api.play.status().then(setSt, () => undefined);
    void api.play.clips().then(setClips, () => undefined);
  }, []);
  useEffect(load, [load]);
  useInterval(() => void api.play.status().then(setSt, () => undefined), 3000);
  useEvent('play:changed', load);
  useEvent('play:clip', load);
  useEvent<{ done: number; total: number }>('play:install', setInstall);
  useEvent<{ text: string }>('play:heard', (p) => setSt((s) => (s ? { ...s, heard: p.text } : s)));

  if (!settings) return null;
  const p = settings.play;
  const set = (patch: Partial<PlaySettings>) => void update({ play: patch });

  const getRecorder = async () => {
    setInstall({ done: 0, total: 0 });
    try {
      await api.play.install();
      toast.success('The recorder is ready', 'Instant replay starts with your next game.');
    } catch (e) {
      toast.error('The recorder didn’t download', errorText(e));
    } finally {
      setInstall(null);
      load();
    }
  };
  const clipNow = async () => {
    setSaving(true);
    try {
      const path = await api.play.clip(p.clipSeconds);
      toast.success('Clip saved', path);
    } catch (e) {
      toast.error('No clip', errorText(e));
    } finally {
      setSaving(false);
    }
  };
  const remove = async (c: PlayClip) => {
    if (!(await confirm({ title: `Delete “${c.name}”?`, description: 'It goes to the Recycle Bin.', tone: 'danger', confirmLabel: 'Delete' }))) return;
    try {
      await api.play.deleteClip(c.path);
      load();
    } catch (e) {
      toast.error('Couldn’t delete it', errorText(e));
    }
  };
  const addName = () => {
    const n = newName.trim().toLowerCase();
    setNewName('');
    if (n && !p.nicknames.includes(n) && !BUILT_IN_NAMES.includes(n)) set({ nicknames: [...p.nicknames, n] });
  };
  const pickFolder = async () => {
    const f = await api.app.pickFolder('Where should clips go?');
    if (f) set({ folder: f });
  };

  return (
    <div className="space-y-4">
      {/* What's happening now */}
      <Card className="flex flex-wrap items-center gap-x-6 gap-y-3 px-5 py-4">
        <div className="min-w-0 flex-1">
          <div className="text-[11px] font-semibold uppercase tracking-[0.08em] text-faint">Now</div>
          <div className="mt-0.5 text-[15px] font-semibold text-fg">{st?.game ? `Playing ${st.game}` : 'No game running'}</div>
          <div className="mt-0.5 flex flex-wrap gap-1.5 text-[12px]">
            {st?.replaying ? (
              <Badge tone="good" icon={Circle}>
                Instant replay · {mmss(st.buffered)} kept{st.encoder ? ` · ${st.encoder}` : ''}
              </Badge>
            ) : (
              <Badge tone="neutral">{p.replay ? (st?.recorderInstalled ? 'Replay starts with a game' : 'Recorder not downloaded') : 'Instant replay off'}</Badge>
            )}
            {st?.recording && <Badge tone="bad">Recording</Badge>}
            <Badge tone={st?.listening ? 'good' : 'neutral'} icon={st?.listening ? Mic : MicOff}>
              {st?.listening ? 'Listening for “Capri”' : p.voice ? 'Voice starts with a game' : 'Voice off'}
            </Badge>
            <Badge tone={st?.capriRunning ? 'good' : 'neutral'} icon={Sparkles}>
              {st?.capriRunning ? 'Capri is here' : 'Capri isn’t running'}
            </Badge>
          </div>
          {st?.heard && <div className="mt-1.5 text-[12px] text-faint">Heard: “{st.heard}”</div>}
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="primary" icon={Clapperboard} loading={saving} disabled={!st?.replaying} onClick={() => void clipNow()}>
            Clip last {p.clipSeconds} s
          </Button>
          <Button icon={st?.recording ? Square : Circle} disabled={!st?.replaying} onClick={() => void api.play.record(!st?.recording).then(load)}>
            {st?.recording ? 'Stop recording' : 'Start recording'}
          </Button>
        </div>
      </Card>
      {st && !st.supported && <Callout tone="info">Instant replay and voice commands work on Windows.</Callout>}
      {st?.replayError && <Callout tone="warn" title="Instant replay">{st.replayError}</Callout>}
      {st?.voiceError && <Callout tone="warn" title="Voice commands">{st.voiceError}</Callout>}

      <div className="grid gap-4 xl:grid-cols-2">
        <Card>
          <CardHeader icon={Clapperboard} title="Instant replay" subtitle="Keeps the last minutes of your screen and game sound while a game runs, so “clip that” can save what just happened. Encoded on your graphics card, so it barely costs FPS." className="px-5 pt-4" actions={<Switch checked={p.replay} onChange={(v) => set({ replay: v })} label="Instant replay" />} />
          <div className="divide-y divide-line px-5 pb-3 pt-1">
            {!st?.recorderInstalled && (
              <div className="py-3">
                <Callout
                  tone="accent"
                  icon={Download}
                  title="Get the recorder once"
                  action={
                    <Button size="sm" variant="primary" icon={Download} loading={!!install} onClick={() => void getRecorder()}>
                      {install?.total ? `${Math.round((install.done / install.total) * 100)}%` : 'Download (about 130 MB)'}
                    </Button>
                  }
                >
                  FFmpeg, published with this OmniHub release and checked against its checksum.
                </Callout>
              </div>
            )}
            <Row title="Keep the last" hint="Kept on disk while you play, deleted as it goes.">
              <Select value={p.replayMinutes} onChange={(e) => set({ replayMinutes: Number(e.target.value) })} className="w-[130px]" aria-label="Minutes kept">
                {[1, 2, 3, 5, 10, 15, 20].map((m) => (
                  <option key={m} value={m}>
                    {m} {m === 1 ? 'minute' : 'minutes'}
                  </option>
                ))}
              </Select>
            </Row>
            <Row title="“Clip that” saves" hint="Say how long for anything else: “Capri, record the last five minutes”.">
              <Select value={p.clipSeconds} onChange={(e) => set({ clipSeconds: Number(e.target.value) })} className="w-[130px]" aria-label="Clip length">
                {[15, 20, 30, 45, 60, 90, 120].map((s) => (
                  <option key={s} value={s}>
                    {s} seconds
                  </option>
                ))}
              </Select>
            </Row>
            <Row title="Quality">
              <Segmented size="sm" label="Quality" value={p.quality} onChange={(v) => set({ quality: v })} options={[{ value: 'standard', label: 'Standard' }, { value: 'high', label: 'High' }]} />
            </Row>
            <Row title="Frame rate">
              <Segmented size="sm" label="Frame rate" value={String(p.fps)} onChange={(v) => set({ fps: Number(v) })} options={[{ value: '30', label: '30 FPS' }, { value: '60', label: '60 FPS' }]} />
            </Row>
            <Row title="Game sound" hint="What your speakers or headphones play.">
              <Switch checked={p.gameAudio} onChange={(v) => set({ gameAudio: v })} label="Game sound" />
            </Row>
            <Row title="Screen">
              <Select value={p.monitor} onChange={(e) => set({ monitor: Number(e.target.value) })} className="w-[130px]" aria-label="Screen">
                <option value={0}>Screen 1</option>
                <option value={1}>Screen 2</option>
                <option value={2}>Screen 3</option>
              </Select>
            </Row>
            <Row title="Clips folder" hint={st?.folder}>
              <div className="flex gap-1">
                <IconButton icon={ExternalLink} label="Open the folder" onClick={() => st && void api.app.openPath(st.folder).catch(() => toast.info('No clips yet'))} />
                <Button size="sm" icon={FolderOpen} onClick={() => void pickFolder()}>
                  Change
                </Button>
              </div>
            </Row>
          </div>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader icon={Sparkles} title="Capri" subtitle="She reacts when a game starts and ends, cheers your clips, and in Counter-Strike 2 and League of Legends follows the match itself." className="px-5 pt-4" actions={<CapriToggle />} />
            <div className="divide-y divide-line px-5 pb-3 pt-1">
              <Row title="React to my games">
                <Switch checked={p.capriReacts} onChange={(v) => set({ capriReacts: v })} label="React to my games" />
              </Row>
              <Row title="Clip big moments by itself" hint="Triple kills and better in CS2 and League (needs instant replay).">
                <Switch checked={p.autoClip} onChange={(v) => set({ autoClip: v })} label="Clip big moments" />
              </Row>
              <Row title="What she calls you" hint="Or say “Capri, call me …”.">
                <div className="flex gap-1.5">
                  <TextInput value={p.nickname} onChange={(e) => set({ nickname: e.target.value })} placeholder="champ" className="w-[150px]" aria-label="What Capri calls you" />
                  <Button size="sm" onClick={() => void api.play.hello().catch((e: unknown) => toast.error('Capri didn’t answer', errorText(e)))}>
                    Test
                  </Button>
                </div>
              </Row>
            </div>
          </Card>

          <Card>
            <CardHeader icon={Mic} title="Voice commands" subtitle="Windows’ own speech recognition, on this PC: it only listens for sentences starting with “Capri”." className="px-5 pt-4" actions={<Switch checked={p.voice} onChange={(v) => set({ voice: v })} label="Voice commands" />} />
            <div className="px-5 pb-4">
              <Row title="Only while a game is running" hint="Otherwise whenever OmniHub runs.">
                <Switch checked={p.voiceOnlyInGames} onChange={(v) => set({ voiceOnlyInGames: v })} label="Only while a game runs" />
              </Row>
              <div className="mt-1 flex flex-wrap gap-1.5">
                {EXAMPLES.map((e) => (
                  <span key={e} className="rounded-full bg-surface-2 px-2.5 py-1 text-[12px] text-dim">
                    “{e}”
                  </span>
                ))}
              </div>
              <Field label="Names for “Capri, call me …”" hint={`Besides ${BUILT_IN_NAMES.slice(0, 6).join(', ')} and more.`} className="mt-4">
                <div className="flex flex-wrap items-center gap-1.5">
                  {p.nicknames.map((n) => (
                    <span key={n} className="inline-flex h-7 items-center gap-1 rounded-full bg-accent-soft pl-2.5 pr-1 text-[12px] text-accent">
                      {n}
                      <button type="button" aria-label={`Remove ${n}`} onClick={() => set({ nicknames: p.nicknames.filter((x) => x !== n) })} className="flex h-5 w-5 items-center justify-center rounded-full hover:bg-accent/20">
                        <X size={11} />
                      </button>
                    </span>
                  ))}
                  <TextInput
                    value={newName}
                    onChange={(e) => setNewName(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') {
                        e.preventDefault();
                        addName();
                      }
                    }}
                    placeholder="Add a name"
                    className="w-[140px]"
                    aria-label="Add a name"
                  />
                  <IconButton icon={Plus} label="Add" size="sm" onClick={addName} />
                </div>
              </Field>
            </div>
          </Card>
        </div>
      </div>

      <Card>
        <CardHeader icon={Clapperboard} title="Your clips" className="px-5 pt-4" />
        <div className="px-3 pb-3 pt-1">
          {clips && clips.length === 0 ? (
            <EmptyState compact icon={Clapperboard} title="No clips yet" description="Turn on instant replay, play, and say “Capri, clip that”." />
          ) : (
            clips?.map((c) => (
              <div key={c.path} className="group flex items-center gap-3 rounded-xl px-3 py-2 hover:bg-surface-2">
                <Clapperboard size={16} className="shrink-0 text-accent" aria-hidden />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[13.5px] font-medium text-fg">{c.name}</div>
                  <div className="text-[12px] text-faint">
                    {formatBytes(c.size)} · {formatRelative(c.created)}
                  </div>
                </div>
                <div className="flex gap-0.5 opacity-70 group-hover:opacity-100">
                  <IconButton icon={ExternalLink} label="Play" size="sm" onClick={() => void api.app.openPath(c.path)} />
                  <IconButton icon={FolderOpen} label="Show in folder" size="sm" onClick={() => void api.app.revealPath(c.path)} />
                  <IconButton icon={Trash} label="Delete" size="sm" variant="danger" onClick={() => void remove(c)} />
                </div>
              </div>
            ))
          )}
        </div>
      </Card>
      <p className="px-1 text-[12px] text-faint">Capri hears OmniHub through her own local link (the one Claude Code uses), so she needs “follow your coding assistant” on in her menu. A game in exclusive full-screen covers her; borderless windowed keeps her in view. Counter-Strike 2 picks up OmniHub’s link the next time it starts.</p>
    </div>
  );
}
