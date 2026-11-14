// Built-in note templates. {{date}}, {{time}}, {{weekday}} and {{week}} are
// filled in when a note is created from one.

import type { NoteKind } from '@shared/types';

export interface NoteTemplate {
  id: string;
  name: string;
  description: string;
  kind: NoteKind;
  title: string;
  body: string;
  tags: string[];
}

export const TEMPLATES: NoteTemplate[] = [
  {
    id: 'meeting',
    name: 'Meeting notes',
    description: 'Who, what was decided, and who does what next.',
    kind: 'note',
    title: 'Meeting — {{date}}',
    tags: ['meeting'],
    body: '## Attendees\n- \n\n## Agenda\n1. \n\n## Notes\n\n\n## Decisions\n- \n\n## Action items\n- [ ] Who — what — by when\n',
  },
  {
    id: 'daily',
    name: 'Daily log',
    description: 'Top three for today, notes, and what to carry over.',
    kind: 'note',
    title: '{{weekday}} {{date}}',
    tags: ['daily'],
    body: '## Top 3 today\n- [ ] \n- [ ] \n- [ ] \n\n## Notes\n\n\n## Done\n- \n\n## Tomorrow\n- \n',
  },
  {
    id: 'todo',
    name: 'To-do list',
    description: 'A checklist, grouped by priority.',
    kind: 'note',
    title: 'To do',
    tags: ['todo'],
    body: '## Now\n- [ ] \n\n## Soon\n- [ ] \n\n## Someday\n- [ ] \n',
  },
  {
    id: 'weekly',
    name: 'Weekly review',
    description: 'Wins, lessons, and the plan for next week.',
    kind: 'note',
    title: 'Week {{week}} review',
    tags: ['weekly'],
    body: '## Wins\n- \n\n## What got in the way\n- \n\n## Lessons\n- \n\n## Next week\n- [ ] \n',
  },
  {
    id: 'bug',
    name: 'Bug report',
    description: 'Steps, expected vs actual — ready to paste into an issue.',
    kind: 'note',
    title: 'Bug: ',
    tags: ['bug'],
    body: '## What happened\n\n\n## Steps to reproduce\n1. \n2. \n3. \n\n## Expected\n\n\n## Actual\n\n\n## Environment\n- OmniHub {{date}}\n- Windows \n',
  },
  {
    id: 'claude-feature',
    name: 'Feature idea for Claude',
    description: 'Goal, context and done-criteria Claude can act on.',
    kind: 'idea',
    title: '',
    tags: ['feature'],
    body: '## Goal\nWhat should exist when this is done?\n\n## Context\nRelevant files, constraints, prior attempts.\n\n## Done when\n- [ ] \n- [ ] \n\n## Out of scope\n- \n',
  },
  {
    id: 'claude-bugfix',
    name: 'Bug for Claude to fix',
    description: 'A failing behaviour with enough detail to fix it.',
    kind: 'idea',
    title: 'Fix: ',
    tags: ['bugfix'],
    body: '## The bug\n\n\n## How to see it\n1. \n\n## Expected\n\n\n## Hints\n- Where it probably lives:\n- Logs / error text:\n',
  },
];

function isoWeek(d: Date): number {
  const t = new Date(Date.UTC(d.getFullYear(), d.getMonth(), d.getDate()));
  const day = t.getUTCDay() || 7;
  t.setUTCDate(t.getUTCDate() + 4 - day);
  const yearStart = new Date(Date.UTC(t.getUTCFullYear(), 0, 1));
  return Math.ceil(((t.getTime() - yearStart.getTime()) / 86400000 + 1) / 7);
}

/** Replace the {{…}} placeholders. */
export function fillTemplate(text: string, now = new Date()): string {
  return text
    .replace(/\{\{date\}\}/g, now.toLocaleDateString(undefined, { day: 'numeric', month: 'short', year: 'numeric' }))
    .replace(/\{\{time\}\}/g, now.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }))
    .replace(/\{\{weekday\}\}/g, now.toLocaleDateString(undefined, { weekday: 'long' }))
    .replace(/\{\{week\}\}/g, String(isoWeek(now)));
}
