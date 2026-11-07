// Mods: every game in the Steam library (and Epic, and added folders) with
// how it's modded, a mod manager per game, Claude making mods, and the
// Minecraft modpack mixer.

import type { ModGame } from '@shared/types';
import { Blocks, FolderPlus, Puzzle, Sparkles, Wand2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Badge, Card } from '../../components/ui/Card';
import { SearchInput, Segmented } from '../../components/ui/Form';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { useEvent } from '../../lib/hooks';
import { navigate, useRoute } from '../../lib/router';
import { toast } from '../../state/toasts';
import { GameView } from './GameView';
import { MinecraftView } from './MinecraftView';
import { ClaudeSettingsModal, GameArt } from './shared';

type Filter = 'all' | 'modded' | 'claude';

export function useModGames() {
  const [games, setGames] = useState<ModGame[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(() => void api.mods.games().then((g) => (setGames(g), setError(null)), (e: unknown) => setError(errorText(e))), []);
  useEffect(load, [load]);
  useEvent('mods:changed', load);
  return { games, error, reload: load };
}

function GameCard({ g }: { g: ModGame }) {
  return (
    <Card interactive className="group overflow-hidden" onClick={() => navigate('mods', { game: g.key })}>
      <GameArt game={g} className="aspect-[460/215] w-full" />
      <div className="space-y-1.5 px-4 py-3">
        <div className="truncate text-[13.5px] font-semibold text-fg">{g.name}</div>
        <div className="flex flex-wrap items-center gap-1.5">
          {g.onlineOnly ? (
            <Badge tone="bad">No mods (online)</Badge>
          ) : (
            <>
              <Badge tone="neutral">{g.engineLabel}</Badge>
              {g.mods > 0 && <Badge tone="good">{g.enabledMods === g.mods ? `${g.mods} mod${g.mods === 1 ? '' : 's'}` : `${g.enabledMods} of ${g.mods} on`}</Badge>}
              {g.thunderstore && <Badge tone="info">Thunderstore</Badge>}
              {g.antiCheat.length > 0 && <Badge tone="warn">Anti-cheat</Badge>}
            </>
          )}
        </div>
      </div>
    </Card>
  );
}

export function ModsPage() {
  const route = useRoute();
  const key = route.params.get('game');
  const { games, error, reload } = useModGames();
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [claudeOpen, setClaudeOpen] = useState(false);

  const shown = useMemo(() => {
    const q = query.trim().toLowerCase();
    return (games ?? []).filter((g) => (!q || g.name.toLowerCase().includes(q)) && (filter === 'all' || (filter === 'modded' ? g.mods > 0 : g.maker.available)));
  }, [games, query, filter]);

  const addFolder = async () => {
    const dir = await api.app.pickFolder('The game’s folder');
    if (!dir) return;
    try {
      const g = await api.mods.addFolder('', dir);
      navigate('mods', { game: g.key });
    } catch (e) {
      toast.error('Not added', errorText(e));
    }
  };

  if (key === 'minecraft') return <MinecraftView />;
  const game = games?.find((g) => g.key === key);
  if (key && game) return <GameView game={game} onClaude={() => setClaudeOpen(true)} claudeModal={<ClaudeSettingsModal open={claudeOpen} onClose={() => setClaudeOpen(false)} />} />;

  return (
    <Page
      title="Mods"
      subtitle="Mods for every game in your library: from Thunderstore, from a file, or made by Claude. Each one switches off and comes out cleanly."
      actions={
        <>
          <Button variant="secondary" icon={FolderPlus} onClick={() => void addFolder()}>
            Add a game folder
          </Button>
          <Button variant="secondary" icon={Sparkles} onClick={() => setClaudeOpen(true)}>
            Claude
          </Button>
        </>
      }
    >
      <ClaudeSettingsModal open={claudeOpen} onClose={() => setClaudeOpen(false)} />
      <div className="space-y-5">
        <Card interactive className="flex items-center gap-4 overflow-hidden px-5 py-4" onClick={() => navigate('mods', { game: 'minecraft' })}>
          <div className="grid h-12 w-12 shrink-0 place-items-center rounded-2xl bg-[linear-gradient(135deg,#3f8f29,#1f5f17)] text-white">
            <Blocks size={24} />
          </div>
          <div className="min-w-0 flex-1">
            <div className="text-[14.5px] font-semibold text-fg">Minecraft: mix modpacks</div>
            <div className="text-[12.5px] text-dim">Put two or more packs together for one Minecraft version: every mod once, updated to fit, with what they need. Save it as a .mrpack or install it in the Minecraft Launcher.</div>
          </div>
          <Wand2 size={18} className="text-accent" />
        </Card>

        <div className="flex flex-wrap items-center gap-3">
          <SearchInput value={query} onChange={setQuery} placeholder="Find a game…" className="w-72" />
          <Segmented
            label="Show"
            value={filter}
            onChange={setFilter}
            options={[
              { value: 'all', label: `All${games ? ` (${games.length})` : ''}` },
              { value: 'modded', label: 'With mods' },
              { value: 'claude', label: 'Claude can mod' },
            ]}
          />
        </div>

        {error && <ErrorState error={error} onRetry={reload} />}
        {games && shown.length === 0 && (
          <Card>
            <EmptyState icon={Puzzle} title={games.length ? 'No game matches' : 'No games found'} description={games.length ? 'Try another name.' : 'Games installed with Steam or Epic appear here. For others, add the game’s folder.'} action={!games.length && <Button onClick={() => void addFolder()}>Add a game folder</Button>} />
          </Card>
        )}
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4">
          {shown.map((g) => (
            <GameCard key={g.key} g={g} />
          ))}
        </div>
      </div>
    </Page>
  );
}
