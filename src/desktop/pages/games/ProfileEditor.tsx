// One game profile: how it starts, what the boost does, the ping helper
// and (for Roblox) its Fast Flags. Changes save on their own.

import type { GameBoost, GameLaunch, GameProfile, GameState, Priority, ProcessGroup } from '@shared/types';
import { Check, FolderOpen, Plus, Rocket, Trash2, X, Zap } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { type MenuItem, openMenu } from '../../components/ui/Menu';
import { cx } from '../../lib/cx';
import { confirm } from '../../state/dialogs';
import { toast } from '../../state/toasts';
import { GameSettingsCard } from './GameSettingsCard';
import { PingCard } from './PingCard';
import { RobloxCard } from './RobloxCard';
import { AdminBadge, GAMES, GameTile, PerGameBadge, Row } from './shared';

const LAUNCH_TYPES: { value: GameLaunch['type']; label: string }[] = [
  { value: 'none', label: 'I start it myself' },
  { value: 'exe', label: 'A program on this PC' },
  { value: 'steam', label: 'Steam game' },
  { value: 'epic', label: 'Epic Games game' },
  { value: 'riot', label: 'Riot game' },
  { value: 'roblox', label: 'Roblox' },
  { value: 'url', label: 'A link (steam://, …)' },
];

function emptyLaunch(type: GameLaunch['type']): GameLaunch {
  switch (type) {
    case 'none':
      return { type };
    case 'exe':
      return { type, path: '', args: '' };
    case 'url':
      return { type, url: '' };
    case 'steam':
      return { type, appId: 0 };
    case 'epic':
      return { type, app: '' };
    case 'riot':
      return { type, product: 'valorant' };
    case 'roblox':
      return { type, placeId: null };
  }
}

/** Programs users commonly close before playing, when they are running. */
const SUGGEST = ['chrome.exe', 'msedge.exe', 'brave.exe', 'firefox.exe', 'opera.exe', 'OneDrive.exe', 'Dropbox.exe', 'GoogleDriveFS.exe', 'Spotify.exe', 'Teams.exe', 'ms-teams.exe', 'Slack.exe', 'qbittorrent.exe', 'uTorrent.exe', 'Creative Cloud.exe', 'Widgets.exe', 'Code.exe'];
const NEVER = new Set(['omnihub.exe', 'explorer.exe', 'steam.exe', 'steamwebhelper.exe', 'epicgameslauncher.exe', 'riotclientservices.exe']);

export function ProfileEditor({ profile, busy, activeHere, onSaved, onDeleted, onPlay }: { profile: GameProfile; busy: boolean; activeHere: boolean; onSaved: (p: GameProfile) => void; onDeleted: () => void; onPlay: (launch: boolean) => void }) {
  const [draft, setDraft] = useState(profile);
  const [state, setState] = useState<GameState | null>(null);
  const [saved, setSaved] = useState<'idle' | 'saving' | 'saved'>('idle');
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const latest = useRef(draft);

  useEffect(() => {
    void api.games.state(profile.id).then(setState, () => undefined);
  }, [profile.id, activeHere]);

  const commit = async (p: GameProfile) => {
    setSaved('saving');
    try {
      const r = await api.games.save(p);
      onSaved(r.profile);
      r.warnings.forEach((w) => toast.warn('Saved with a note', w));
      setSaved('saved');
      // Keep any edits made while saving.
      if (latest.current === p) setDraft(r.profile);
    } catch (e) {
      setSaved('idle');
      toast.error('Not saved', errorText(e));
    }
  };
  const change = (p: GameProfile, now = false) => {
    setDraft(p);
    latest.current = p;
    clearTimeout(timer.current);
    timer.current = setTimeout(() => void commit(p), now ? 0 : 600);
  };
  useEffect(() => () => clearTimeout(timer.current), []);
  const boost = (b: Partial<GameBoost>) => change({ ...draft, boost: { ...draft.boost, ...b } });

  const remove = async () => {
    const ok = await confirm({ title: `Delete ${draft.name}?`, description: 'Its per-game settings (GPU choice, fullscreen, network and start priority) are taken off too.', tone: 'danger', confirmLabel: 'Delete' });
    if (!ok) return;
    try {
      await api.games.remove(draft.id);
      onDeleted();
    } catch (e) {
      toast.error('Could not delete', errorText(e));
    }
  };

  const pickExe = async () => {
    const [f] = await api.app.pickFiles('Choose the game’s program');
    if (!f) return;
    const name = f.split(/[\\/]/).pop() ?? '';
    change({ ...draft, launch: { type: 'exe', path: f, args: draft.launch.type === 'exe' ? draft.launch.args : '' }, process: draft.process || name, exePath: f }, true);
  };

  const addApp = async (e: React.MouseEvent) => {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    let running: ProcessGroup[] = [];
    try {
      running = (await api.app.usage('memory', 80)).processes.filter((p) => p.canEnd && !NEVER.has(p.name.toLowerCase()) && p.name.toLowerCase() !== draft.process.toLowerCase());
    } catch {
      /* show suggestions only */
    }
    const chosen = new Set(draft.boost.closeApps.map((a) => a.toLowerCase()));
    const fmt = (b: number) => `${(b / 1024 / 1024 / 1024).toFixed(1)} GB`;
    const items: MenuItem[] = [{ kind: 'header', label: 'Running now' }];
    running
      .filter((p) => !chosen.has(p.name.toLowerCase()))
      .slice(0, 14)
      .forEach((p) => items.push({ label: p.name.replace(/\.exe$/i, ''), hint: p.memory > 200 * 1024 * 1024 ? fmt(p.memory) : undefined, onSelect: () => boost({ closeApps: [...draft.boost.closeApps, p.name] }) }));
    const extra = SUGGEST.filter((s) => !chosen.has(s.toLowerCase()) && !running.some((p) => p.name.toLowerCase() === s.toLowerCase()));
    if (extra.length) {
      items.push({ kind: 'separator' }, { kind: 'header', label: 'Common' });
      extra.slice(0, 8).forEach((s) => items.push({ label: s.replace(/\.exe$/i, ''), onSelect: () => boost({ closeApps: [...draft.boost.closeApps, s] }) }));
    }
    openMenu({ clientX: r.left, clientY: r.bottom + 4, preventDefault: () => undefined }, items);
  };

  const l = draft.launch;
  const b = draft.boost;
  return (
    <div className="space-y-4">
      <Card className="flex flex-wrap items-center gap-4 p-5">
        <GameTile kind={draft.kind} size={56} />
        <div className="min-w-0 flex-1">
          <input value={draft.name} onChange={(e) => change({ ...draft, name: e.target.value })} className="w-full bg-transparent font-display text-[20px] font-semibold text-fg outline-none" aria-label="Profile name" />
          <div className="flex min-w-0 items-center gap-2 text-[12.5px] text-faint">
            <span className="shrink-0">{GAMES[draft.kind].label}</span>
            {draft.process && <span className="min-w-0 truncate font-mono text-[11.5px]" title={draft.process}>· {draft.process}</span>}
            <span className={cx('ml-1 shrink-0 whitespace-nowrap transition-opacity', saved === 'idle' && 'opacity-0')}>{saved === 'saving' ? 'Saving…' : <span className="inline-flex items-center gap-1 text-good"><Check size={12} /> Saved</span>}</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="secondary" icon={Zap} disabled={busy} onClick={() => onPlay(false)} title="Apply the boost now; start the game yourself">
            Boost only
          </Button>
          <Button icon={Rocket} disabled={busy || l.type === 'none'} onClick={() => onPlay(true)} title={l.type === 'none' ? 'Choose how the game starts below' : 'Boost, then start the game'}>
            {activeHere ? 'Boosted' : 'Play'}
          </Button>
          <IconButton icon={Trash2} variant="danger" label="Delete profile" onClick={() => void remove()} disabled={activeHere} />
        </div>
      </Card>

      {(draft.kind === 'fortnite' || draft.kind === 'minecraft') && <GameSettingsCard game={draft.kind} beforeLaunch={b.gameSettings} onBeforeLaunch={(v) => boost({ gameSettings: v })} />}

      <Card>
        <CardHeader title="Start" subtitle="How OmniHub starts the game, and which program to follow" className="px-5 pt-4" />
        <div className="divide-y divide-line">
          <Row title="Starts with">
            <Select value={l.type} onChange={(e) => change({ ...draft, launch: emptyLaunch(e.target.value as GameLaunch['type']) })} className="w-56">
              {LAUNCH_TYPES.map((t) => (
                <option key={t.value} value={t.value}>
                  {t.label}
                </option>
              ))}
            </Select>
          </Row>
          {l.type === 'exe' && (
            <Row title="Program" stack>
              <div className="flex gap-2">
                <TextInput value={l.path} onChange={(e) => change({ ...draft, launch: { ...l, path: e.target.value } })} placeholder="C:\Games\MyGame\Game.exe" className="flex-1 font-mono text-[12.5px]" />
                <Button variant="secondary" icon={FolderOpen} onClick={() => void pickExe()}>
                  Browse
                </Button>
              </div>
              <TextInput value={l.args} onChange={(e) => change({ ...draft, launch: { ...l, args: e.target.value } })} placeholder="Arguments (optional)" className="font-mono text-[12.5px]" />
            </Row>
          )}
          {l.type === 'steam' && (
            <Row title="Steam app ID" hint="The number in the game's store link: store.steampowered.com/app/730 → 730">
              <TextInput value={l.appId || ''} inputMode="numeric" onChange={(e) => change({ ...draft, launch: { ...l, appId: Number(e.target.value.replace(/\D/g, '')) || 0 } })} className="w-36" />
            </Row>
          )}
          {l.type === 'epic' && (
            <Row title="Epic app name" hint="Fortnite and Rocket League are filled in for you.">
              <TextInput value={l.app} onChange={(e) => change({ ...draft, launch: { ...l, app: e.target.value } })} className="w-72 font-mono text-[12px]" />
            </Row>
          )}
          {l.type === 'riot' && (
            <Row title="Riot game">
              <Select value={l.product} onChange={(e) => change({ ...draft, launch: { ...l, product: e.target.value } })} className="w-56">
                <option value="valorant">VALORANT</option>
                <option value="league_of_legends">League of Legends</option>
                <option value="bacon">Legends of Runeterra</option>
              </Select>
            </Row>
          )}
          {l.type === 'roblox' && (
            <Row title="Join an experience" hint="Optional: the number in its link, roblox.com/games/920587237 → 920587237. Empty opens the Roblox app.">
              <TextInput value={l.placeId ?? ''} inputMode="numeric" placeholder="Place ID" onChange={(e) => change({ ...draft, launch: { ...l, placeId: Number(e.target.value.replace(/\D/g, '')) || null } })} className="w-40" />
            </Row>
          )}
          {l.type === 'url' && (
            <Row title="Link" stack>
              <TextInput value={l.url} onChange={(e) => change({ ...draft, launch: { ...l, url: e.target.value } })} placeholder="steam://rungameid/730" className="font-mono text-[12.5px]" />
            </Row>
          )}
          <Row title="Game program" hint="The game's own .exe. OmniHub follows it to know when you stop playing, and gives it priority.">
            <TextInput value={draft.process} onChange={(e) => change({ ...draft, process: e.target.value })} placeholder="Game.exe" className="w-72 font-mono text-[12.5px]" />
          </Row>
        </div>
      </Card>

      <Card>
        <CardHeader title="Boost" subtitle="Applied when you press Play; put back when the game closes" className="px-5 pt-4" />
        <div className="divide-y divide-line">
          <Row title="Power plan" hint="Ultimate Performance keeps the processor at full speed (OmniHub adds Windows' hidden plan the first time). Your plan comes back afterwards.">
            <Segmented
              size="sm"
              label="Power plan"
              value={b.powerPlan}
              onChange={(v) => boost({ powerPlan: v })}
              options={[
                { value: 'keep', label: 'Keep' },
                { value: 'high', label: 'High' },
                { value: 'ultimate', label: 'Ultimate' },
              ]}
            />
          </Row>
          <Row title="Game priority" hint="Windows gives the game the processor first while it runs.">
            <Select value={b.priority ?? ''} onChange={(e) => boost({ priority: (e.target.value || null) as Priority | null })} className="w-40">
              <option value="">Leave as is</option>
              <option value="aboveNormal">Above normal</option>
              <option value="high">High</option>
            </Select>
          </Row>
          <Row title="Close while playing" hint="These programs are closed for the session — unsaved work in them is lost. Browsers usually offer to restore their tabs." stack>
            <div className="flex flex-wrap items-center gap-1.5">
              {b.closeApps.map((a) => (
                <span key={a} className="inline-flex items-center gap-1 rounded-full border border-line bg-surface-2 py-0.5 pl-2.5 pr-1 text-[12.5px] text-fg">
                  {a.replace(/\.exe$/i, '')}
                  <button type="button" onClick={() => boost({ closeApps: b.closeApps.filter((x) => x !== a) })} className="grid h-5 w-5 place-items-center rounded-full text-faint hover:bg-surface-3 hover:text-fg" aria-label={`Keep ${a} open`}>
                    <X size={12} />
                  </button>
                </span>
              ))}
              <Button size="sm" variant="ghost" icon={Plus} onClick={(e) => void addApp(e)}>
                Add
              </Button>
            </div>
            {b.closeApps.length > 0 && (
              <label className="flex items-center gap-2 text-[12.5px] text-dim">
                <Switch checked={b.reopenApps} onChange={(v) => boost({ reopenApps: v })} label="Reopen them afterwards" /> Reopen them when the game closes
              </label>
            )}
          </Row>
          <Row title="Silence notifications" hint="No pop-ups over the game; they come back afterwards.">
            <Switch checked={b.silenceNotifications} onChange={(v) => boost({ silenceNotifications: v })} label="Silence notifications" />
          </Row>
          <Row title="Game Mode" hint="Makes sure Windows Game Mode is on (it holds back updates and background work).">
            <Switch checked={b.gameMode} onChange={(v) => boost({ gameMode: v })} label="Game Mode" />
          </Row>
          <Row title="Use the high-performance GPU" badge={<PerGameBadge />} hint="Same as Windows Settings → Graphics. Matters on laptops with two GPUs.">
            <Switch checked={b.gpuHighPerformance} onChange={(v) => boost({ gpuHighPerformance: v })} label="High-performance GPU" />
          </Row>
          <Row title="Disable fullscreen optimizations" badge={<PerGameBadge />} hint="Can lower input delay in some older games; leave off if the game alt-tabs badly.">
            <Switch checked={b.fullscreenOptimizationsOff} onChange={(v) => boost({ fullscreenOptimizationsOff: v })} label="Disable fullscreen optimizations" />
          </Row>
          <Row title="Always start at High priority" badge={<AdminBadge on={state?.startHighPriority} />} hint="Windows itself starts the game at High priority — works even when anti-cheat blocks changing it later (Fortnite, VALORANT).">
            <Switch checked={b.startHighPriority} onChange={(v) => boost({ startHighPriority: v })} label="Always start at High priority" />
          </Row>
          {!draft.exePath && (b.gpuHighPerformance || b.fullscreenOptimizationsOff) && <div className="px-5 py-2.5 text-[12px] text-faint">GPU and fullscreen settings are applied once OmniHub has seen the game run.</div>}
        </div>
      </Card>

      <PingCard profile={draft} state={state} onBoost={boost} onHost={(h) => change({ ...draft, pingHost: h })} />

      {draft.kind === 'roblox' && <RobloxCard flags={draft.roblox} onChange={(f) => change({ ...draft, roblox: f })} />}
    </div>
  );
}
