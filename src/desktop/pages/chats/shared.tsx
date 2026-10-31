// What the Chats views share: how each kind of chat is named and drawn,
// the modes Claude Code and Codex run in, and short number formats.

import type { ChatSource, ChatSummary, ClaudeCodeMode, CodexSandbox, NewChatKind } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { Bot, Code2, Sparkles, SquareTerminal } from 'lucide-react';
import { cx } from '../../lib/cx';

export interface Kind {
  label: string;
  icon: LucideIcon;
  /** Tailwind classes for the round badge. */
  tint: string;
}

export const KINDS: Record<'claude' | 'openai' | 'claudeCode' | 'codex', Kind> = {
  claude: { label: 'Claude', icon: Sparkles, tint: 'bg-[#d97757]/15 text-[#d97757]' },
  openai: { label: 'ChatGPT', icon: Bot, tint: 'bg-emerald-500/15 text-emerald-500' },
  claudeCode: { label: 'Claude Code', icon: SquareTerminal, tint: 'bg-amber-500/15 text-amber-500' },
  codex: { label: 'Codex', icon: Code2, tint: 'bg-sky-500/15 text-sky-500' },
};

export function kindOf(s: Pick<ChatSummary, 'source' | 'provider'>): Kind {
  if (s.source === 'omni') return s.provider === 'openai' ? KINDS.openai : KINDS.claude;
  return KINDS[s.source];
}

export function KindIcon({ s, size = 32, className }: { s: Pick<ChatSummary, 'source' | 'provider'>; size?: number; className?: string }) {
  const k = kindOf(s);
  const Icon = k.icon;
  return (
    <div className={cx('grid shrink-0 place-items-center rounded-full', k.tint, className)} style={{ width: size, height: size }}>
      <Icon size={Math.round(size * 0.5)} />
    </div>
  );
}

export const CLAUDE_MODES: { id: ClaudeCodeMode; label: string; hint: string }[] = [
  { id: 'plan', label: 'Plan only', hint: 'Reads and plans; changes nothing.' },
  { id: 'default', label: 'Ask first', hint: 'Only what your Claude Code settings already allow; anything else is skipped and listed.' },
  { id: 'acceptEdits', label: 'Edit files', hint: 'Edits files in the project folder; other commands need your Claude Code rules.' },
  { id: 'auto', label: 'Auto', hint: 'Claude Code’s safety check decides each step.' },
];

export const CODEX_SANDBOXES: { id: CodexSandbox; label: string; hint: string }[] = [
  { id: 'read-only', label: 'Read only', hint: 'Looks and answers; changes nothing.' },
  { id: 'workspace-write', label: 'Edit the project', hint: 'Can change files inside the project folder only.' },
];

export function sourceOfKind(k: NewChatKind): ChatSource {
  return k === 'claudeCode' ? 'claudeCode' : k === 'codex' ? 'codex' : 'omni';
}

export function folderName(path: string | null): string {
  if (!path) return '';
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** 1234 → "1.2k", 4_500_000 → "4.5M". */
export function tokensShort(n: number): string {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)}B`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)}M`;
  if (n >= 1e4) return `${Math.round(n / 1e3)}k`;
  if (n >= 1e3) return `${(n / 1e3).toFixed(1)}k`;
  return `${n}`;
}

export function dollars(n: number): string {
  if (n === 0) return '$0';
  return n < 0.01 ? '<$0.01' : n >= 100 ? `$${Math.round(n)}` : `$${n.toFixed(2)}`;
}

/** "in 2 h 10 min", "in 3 days". */
export function untilText(unix: number): string {
  const s = unix - Math.floor(Date.now() / 1000);
  if (s <= 60) return 'in a minute';
  if (s < 3600) return `in ${Math.round(s / 60)} min`;
  if (s < 86_400) {
    const h = Math.floor(s / 3600);
    const m = Math.round((s % 3600) / 60);
    return m ? `in ${h} h ${m} min` : `in ${h} h`;
  }
  const d = Math.round(s / 86_400);
  return `in ${d} day${d === 1 ? '' : 's'}`;
}
