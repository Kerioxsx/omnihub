// Usage: Claude Code's (from its own logs on this PC, with what the same
// tokens would cost on the API and its 5-hour window), Codex's (its plan
// limits as Codex last reported them) and OmniHub's own with your API keys.

import type { AgentUsage, UsageDay, UsagePeriod, UsageReport, UsageTokens } from '@shared/types';
import { formatRelative } from '@shared/format';
import { AlertTriangle, ExternalLink, Gauge, RefreshCw } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Card, Spinner } from '../../components/ui/Card';
import { Callout, ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { dollars, KindIcon, tokensShort, untilText } from './shared';

const total = (t: UsageTokens) => t.input + t.output + t.cacheRead + t.cacheWrite;

function Stat({ label, tokens, priced, note }: { label: string; tokens: UsageTokens; priced: boolean; note?: string }) {
  return (
    <div className="rounded-xl bg-surface-2/50 px-3.5 py-2.5">
      <div className="text-[11px] font-medium uppercase tracking-wide text-faint">{label}</div>
      <div className="mt-0.5 text-[18px] font-semibold tabular-nums text-fg">{priced ? dollars(tokens.cost) : tokensShort(total(tokens))}</div>
      <div className="text-[11.5px] tabular-nums text-dim">{priced ? `${tokensShort(total(tokens))} tokens` : `${tokensShort(tokens.output)} written`}{note ? ` · ${note}` : ''}</div>
    </div>
  );
}

/** The last 30 days, one bar a day; days without use stay empty. */
function DayBars({ days, priced, title }: { days: UsageDay[]; priced: boolean; title: string }) {
  const [hover, setHover] = useState<number | null>(null);
  const slots = useMemo(() => {
    const by = new Map(days.map((d) => [d.date, d]));
    return Array.from({ length: 30 }, (_, i) => {
      const d = new Date(Date.now() - (29 - i) * 86_400_000);
      const key = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
      const day = by.get(key);
      return { key, label: d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' }), value: day ? (priced ? day.cost : total(day)) : 0, day };
    });
  }, [days, priced]);
  const max = Math.max(...slots.map((s) => s.value), 0) || 1;
  const fmt = (v: number) => (priced ? dollars(v) : `${tokensShort(v)} tokens`);
  const h = hover === null ? null : slots[hover];
  return (
    <figure className="space-y-1.5">
      <figcaption className="text-[12px] font-medium text-dim">{title}</figcaption>
      <div className="relative">
        <div className="flex h-24 items-end gap-[2px] border-b border-line" onMouseLeave={() => setHover(null)}>
          {slots.map((s, i) => (
            <div key={s.key} className="flex h-full flex-1 items-end" onMouseEnter={() => setHover(i)} role="img" aria-label={`${s.label}: ${fmt(s.value)}`}>
              <div className={cx('w-full rounded-t-[4px] bg-accent transition-opacity', hover !== null && hover !== i && 'opacity-50')} style={{ height: s.value ? `${Math.max(3, (s.value / max) * 100)}%` : 0 }} />
            </div>
          ))}
        </div>
        {h && (
          <div className="pointer-events-none absolute -top-9 z-10 -translate-x-1/2 whitespace-nowrap rounded-lg border border-line bg-surface px-2.5 py-1 text-[11.5px] text-fg shadow-lg" style={{ left: `${((hover ?? 0) + 0.5) * (100 / 30)}%` }}>
            <span className="text-dim">{h.label}</span> · <span className="tabular-nums">{fmt(h.value)}</span>
          </div>
        )}
      </div>
      <div className="flex justify-between text-[11px] text-faint">
        <span>{slots[0].label}</span>
        <span>Today</span>
      </div>
      <details className="text-[11.5px] text-dim">
        <summary className="cursor-pointer text-faint hover:text-dim">As a table</summary>
        <table className="mt-1.5 w-full tabular-nums">
          <thead>
            <tr className="text-left text-faint">
              <th className="font-medium">Day</th>
              <th className="text-right font-medium">Tokens</th>
              {priced && <th className="text-right font-medium">API value</th>}
            </tr>
          </thead>
          <tbody>
            {slots
              .filter((s) => s.day)
              .reverse()
              .map((s) => (
                <tr key={s.key}>
                  <td>{s.label}</td>
                  <td className="text-right">{tokensShort(total(s.day as UsageTokens))}</td>
                  {priced && <td className="text-right">{dollars(s.day?.cost ?? 0)}</td>}
                </tr>
              ))}
          </tbody>
        </table>
      </details>
    </figure>
  );
}

function Meter({ name, used, resetsAt, reached }: { name: string; used: number | null; resetsAt: number | null; reached: boolean }) {
  const pct = used === null ? null : Math.min(100, Math.max(0, used));
  const tone = reached || (pct ?? 0) >= 100 ? 'bg-bad' : (pct ?? 0) >= 80 ? 'bg-warn' : 'bg-accent';
  return (
    <div className="space-y-1">
      <div className="flex items-baseline justify-between gap-2 text-[12.5px]">
        <span className="font-medium text-fg">{name}</span>
        <span className="tabular-nums text-dim">{pct === null ? 'Reset since Codex last checked' : `${Math.round(pct)}% used`}</span>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-surface-2">{pct !== null && <div className={cx('h-full rounded-full', tone)} style={{ width: `${pct}%` }} />}</div>
      {resetsAt && <div className="text-[11.5px] text-faint">Resets {untilText(resetsAt)} ({new Date(resetsAt * 1000).toLocaleString(undefined, { weekday: 'short', hour: 'numeric', minute: '2-digit' })})</div>}
    </div>
  );
}

function Period({ p, priced, note }: { p: UsagePeriod; priced: boolean; note?: string }) {
  return (
    <div className="grid gap-2 sm:grid-cols-3">
      <Stat label="Today" tokens={p.today} priced={priced} />
      <Stat label="Last 7 days" tokens={p.week} priced={priced} />
      <Stat label="Last 30 days" tokens={p.month} priced={priced} note={note} />
    </div>
  );
}

function ClaudeCodeCard({ u }: { u: AgentUsage }) {
  const reached = u.limits.filter((l) => l.reached);
  return (
    <Card className="space-y-4 px-5 py-4">
      <div className="flex items-center gap-3">
        <KindIcon s={{ source: 'claudeCode', provider: null }} size={34} />
        <div className="min-w-0 flex-1">
          <div className="text-[15px] font-semibold text-fg">Claude Code</div>
          <div className="text-[12px] text-dim">Added up from Claude Code’s own logs on this PC. With a Claude plan you don’t pay per token: the dollars are what the same use would cost on the API.</div>
        </div>
        <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://claude.ai/settings/usage')}>
          Your plan’s limits
        </Button>
      </div>
      {!u.found ? (
        <div className="text-[12.5px] text-faint">No Claude Code conversations on this PC.</div>
      ) : (
        <>
          {reached.map((l) => (
            <Callout key={l.name} tone="warn" icon={AlertTriangle} title={`${l.name} reached`}>
              {l.resetsAt ? `It resets ${untilText(l.resetsAt)} (${new Date(l.resetsAt * 1000).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}).` : 'Claude Code noted it when you reached it.'}
            </Callout>
          ))}
          {u.window ? (
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-xl border border-accent/30 bg-accent-soft/40 px-4 py-2.5 text-[12.5px]">
              <span className="inline-flex items-center gap-1.5 font-medium text-fg">
                <Gauge size={14} className="text-accent" /> This 5-hour window
              </span>
              <span className="text-dim">
                started about {new Date(u.window.start * 1000).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' })}, ends {untilText(u.window.end)}
              </span>
              <span className="tabular-nums text-dim">
                {tokensShort(total(u.window.tokens))} tokens · {dollars(u.window.tokens.cost)} API value
              </span>
            </div>
          ) : (
            <div className="text-[12px] text-faint">No use in the last 5 hours, so a new window starts with your next message.</div>
          )}
          <Period p={u.period} priced />
          <DayBars days={u.period.days} priced title="API value per day, last 30 days" />
        </>
      )}
    </Card>
  );
}

function CodexCard({ u }: { u: AgentUsage }) {
  return (
    <Card className="space-y-4 px-5 py-4">
      <div className="flex items-center gap-3">
        <KindIcon s={{ source: 'codex', provider: null }} size={34} />
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 text-[15px] font-semibold text-fg">
            Codex {u.plan && <Badge tone="neutral">{u.plan.charAt(0).toUpperCase() + u.plan.slice(1)} plan</Badge>}
          </div>
          <div className="text-[12px] text-dim">Codex writes your plan’s limits into its logs each time it answers{u.limitsAt ? `; last ${formatRelative(u.limitsAt)}` : ''}.</div>
        </div>
        <Button size="sm" variant="secondary" icon={ExternalLink} onClick={() => void api.app.openUrl('https://chatgpt.com/codex/settings/usage')}>
          Codex usage
        </Button>
      </div>
      {!u.found ? (
        <div className="text-[12.5px] text-faint">No Codex conversations on this PC.</div>
      ) : (
        <>
          {u.limits.length > 0 && (
            <div className="grid gap-4 sm:grid-cols-2">
              {u.limits.map((l) => (
                <Meter key={l.name} name={l.name} used={l.usedPercent} resetsAt={l.resetsAt} reached={l.reached} />
              ))}
            </div>
          )}
          <Period p={u.period} priced={false} />
          <DayBars days={u.period.days} priced={false} title="Tokens per day, last 30 days" />
        </>
      )}
    </Card>
  );
}

export function UsageTab() {
  const [r, setR] = useState<UsageReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const load = useCallback(() => {
    setLoading(true);
    void api.chats
      .usage()
      .then((x) => (setR(x), setError(null)), (e: unknown) => setError(errorText(e)))
      .finally(() => setLoading(false));
  }, []);
  useEffect(load, [load]);
  if (error) return <ErrorState error={error} onRetry={load} />;
  if (!r)
    return (
      <div className="flex justify-center py-16">
        <Spinner />
      </div>
    );
  return (
    <div className="mx-auto max-w-4xl space-y-4">
      <div className="flex justify-end">
        <Button size="sm" variant="ghost" icon={RefreshCw} disabled={loading} onClick={load}>
          {loading ? 'Adding up…' : 'Refresh'}
        </Button>
      </div>
      <ClaudeCodeCard u={r.claudeCode} />
      <CodexCard u={r.codex} />
      <Card className="space-y-4 px-5 py-4">
        <div>
          <div className="text-[15px] font-semibold text-fg">OmniHub, with your API keys</div>
          <div className="text-[12px] text-dim">These are billed to your keys. Claude’s cost is OmniHub’s estimate; your provider’s page has the exact bill.</div>
        </div>
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-[13px] font-medium text-fg">
            <KindIcon s={{ source: 'omni', provider: 'claude' }} size={22} /> Claude chats
          </div>
          <Period p={r.omni.claude} priced />
        </div>
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-[13px] font-medium text-fg">
            <KindIcon s={{ source: 'omni', provider: 'openai' }} size={22} /> ChatGPT chats
          </div>
          <Period p={r.omni.openai} priced={false} />
        </div>
        <div className="flex flex-wrap items-center justify-between gap-2 rounded-xl bg-surface-2/50 px-3.5 py-2.5 text-[12.5px]">
          <span className="text-dim">
            Mods made by Claude, all time: <span className="font-medium tabular-nums text-fg">{dollars(r.omni.maker.cost)}</span> ({tokensShort(total(r.omni.maker))} tokens)
          </span>
          <span className="flex gap-2">
            <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://console.anthropic.com/settings/usage')}>
              Anthropic usage
            </Button>
            <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://platform.openai.com/usage')}>
              OpenAI usage
            </Button>
          </span>
        </div>
      </Card>
      <div className="px-1 text-[11.5px] text-faint">Chats in the ChatGPT and Claude apps live in your accounts online, not on this PC, so they aren’t counted here; their usage pages above have them.</div>
    </div>
  );
}
