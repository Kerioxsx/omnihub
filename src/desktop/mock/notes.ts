// Notes, ideas for Claude and the watched Claude folder.

import type { FolderFile, Note, NoteFilter, NoteInput } from '@shared/types';
import { emit } from './bus';
import { settings } from './core';
import { DAY, NOW } from './rng';

let seq = 0;
const nid = () => `note-${(++seq).toString(36)}-${Date.now().toString(36).slice(-4)}`;
const FOLDER = 'C:\\Users\\Alex\\Documents\\Claude Ideas';

type Seed = [kind: Note['kind'], title: string, body: string, tags: string[], pinned: boolean, color: string | null, createdAgo: number, updatedAgo: number, exported: string | null, exportedAgo: number | null];

const SEEDS: Seed[] = [
  [
    'idea',
    'Treemap keyboard navigation',
    `Let me move through the storage treemap without a mouse.

## Behaviour
- **Arrow keys** move the focus ring to the nearest tile in that direction
- **Enter** drills into the focused folder, **Backspace** goes up
- \`Shift+Delete\` opens the delete dialog for the focused item

## Open questions
- [ ] Should focus follow hover, or stay independent?
- [ ] Announce tile name + size for screen readers (aria-live)
- [x] Keep the zoom animation under 250 ms`,
    ['storage', 'a11y'],
    true,
    'violet',
    0.9,
    0.4,
    null,
    null,
  ],
  [
    'idea',
    'Storage treemap zoom',
    `Animate the treemap when drilling into a folder so it feels spatial.

1. Remember the clicked tile's rectangle
2. Lay out the child folder at full size
3. Interpolate every rectangle from the old tile to its new position (~250 ms, ease-out)

\`\`\`ts
const t = ease(Math.min(1, (now - start) / 250));
rect.x = lerp(from.x, to.x, t);
\`\`\`

Folded "N smaller items" tiles should fade instead of moving.`,
    ['storage', 'ux'],
    false,
    'cyan',
    14,
    12,
    `${FOLDER}\\2026-09-21-storage-treemap-zoom.md`,
    12,
  ],
  [
    'idea',
    'Phone companion onboarding flow',
    `Make pairing a phone take under 20 seconds.

- Big QR + 6-digit PIN side by side
- Explain the **one-time certificate warning** before it happens (self-signed HTTPS)
- After pairing, show what the phone can do with toggles right there

> Default everything risky to off: remote control, vault access.`,
    ['phone', 'onboarding'],
    false,
    null,
    6,
    5,
    `${FOLDER}\\2026-09-28-phone-companion-onboarding.md`,
    5,
  ],
  [
    'idea',
    'Vault: passkeys migration plan',
    `Nudge people away from storing primary account passwords.

- Flag Google / Microsoft / Apple logins with an amber banner
- Link to each provider's passkey setup page
- Offer to generate an **app password** entry instead

| Provider | Passkeys | App passwords |
|---|---|---|
| Google | yes | yes |
| Microsoft | yes | yes |
| Apple | yes | yes |`,
    ['vault', 'security'],
    false,
    'amber',
    5,
    4,
    `${FOLDER}\\2026-09-29-vault-passkeys-plan.md`,
    4,
  ],
  [
    'idea',
    'Auto-tag screenshots by window title',
    `Use the foreground window title at capture time to suggest tags.

- \`Visual Studio Code\` → **dev**
- Steam game exe → **game** + the game's name
- Browser + "Order confirmation" → **receipt**

Let the user accept suggestions with one click.`,
    ['screenshots'],
    false,
    null,
    3,
    2.6,
    null,
    null,
  ],
  [
    'idea',
    'Perceptual hashing for duplicate photos',
    `The duplicate finder only catches byte-identical files. Photos exported twice at different quality are missed.

- dHash (64-bit) per image, Hamming distance ≤ 6
- Only for jpg/png/heic over 200 KB
- Show side-by-side preview before deleting`,
    ['storage', 'photos'],
    false,
    'emerald',
    9,
    8,
    null,
    null,
  ],
  ['idea', 'Weekly cleanup digest', 'Every Sunday: a notification with how much space the safe cleanup suggestions would free, e.g. *"14.2 GB of caches and temp files can go"*. One click opens the Cleanup tab.', ['storage', 'notifications'], false, null, 18, 18, null, null],
  [
    'note',
    'Weekend build: NAS upgrade',
    `## Parts
- [x] 2× WD Red Plus 8 TB
- [x] Noctua NF-A12x25
- [ ] SATA cables (short, right-angled)
- [ ] 16 GB ECC stick

## Steps
1. Scrub the pool before swapping disks
2. Replace one disk at a time, resilver
3. Expand the vdev

\`zpool status -v tank\``,
    ['homelab', 'todo'],
    true,
    'cyan',
    10,
    1.2,
    null,
    null,
  ],
  ['note', 'Groceries', '- [ ] Miso paste\n- [ ] Ramen noodles\n- [x] Spring onions\n- [ ] Eggs (12)\n- [ ] Chili crisp\n- [x] Oat milk', ['home'], false, 'emerald', 2, 0.2, null, null],
  [
    'note',
    'Q4 roadmap — meeting notes',
    `**Attendees:** Sam, Priya, Jordan, me

### Decisions
- Ship the storage cleanup v2 before Nov 15
- Mobile companion goes to beta with **HTTPS on by default**

### Action items
- [ ] Alex: write the cleanup risk copy
- [ ] Priya: threat model for remote control
- [ ] Jordan: perf budget for the treemap (60 fps at 1500 tiles)`,
    ['work', 'meetings'],
    false,
    'violet',
    4,
    4,
    null,
    null,
  ],
  ['note', 'Books to read', '1. *The Pragmatic Programmer* (20th anniv.)\n2. *Designing Data-Intensive Applications*\n3. *Project Hail Mary*\n4. *The Design of Everyday Things*\n5. *Piranesi*', ['reading'], false, null, 30, 21, null, null],
  ['note', 'Mesh Wi-Fi placement', 'Node 1: living room shelf (wired backhaul)\nNode 2: upstairs hallway — **not** behind the mirror\nNode 3: office, 5 GHz only\n\nChannel 36/80 MHz performs best; avoid DFS channels (radar drops).', ['homelab'], false, 'amber', 40, 33, null, null],
  ['note', 'Miso ramen', '### Broth\n- 1 L chicken stock\n- 3 tbsp white miso\n- 1 tbsp tahini\n- garlic, ginger, chili oil\n\nSimmer 10 min, whisk in miso **off the heat**.', ['recipes'], false, 'rose', 60, 45, null, null],
  ['note', 'Keyboard shortcuts', '| Action | Keys |\n|---|---|\n| Command palette | `Ctrl+K` |\n| Region screenshot | `Alt+Shift+S` |\n| Switch page | `Ctrl+1…9` |\n| Close dialog | `Esc` |', ['reference'], false, null, 12, 7, null, null],
];

const notes: Note[] = SEEDS.map(([kind, title, body, tags, pinned, color, c, u, exported, exAgo]) => ({
  id: nid(),
  kind,
  title,
  body,
  tags,
  pinned,
  color,
  created: Math.floor(NOW - c * DAY),
  updated: Math.floor(NOW - u * DAY),
  exportedPath: exported,
  exportedAt: exAgo == null ? null : Math.floor(NOW - exAgo * DAY),
}));

// ---------- Claude folder ----------

interface FileEntry {
  name: string;
  content: string;
  modified: number;
  fromOmnihub: boolean;
}

const files = new Map<string, FileEntry>();

function renderMarkdown(n: Note): string {
  const tags = n.tags.length ? `tags: [${n.tags.join(', ')}]\n` : '';
  return `---\ntitle: ${n.title}\nkind: ${n.kind}\n${tags}created: ${new Date(n.created * 1000).toISOString()}\nsource: OmniHub\n---\n\n# ${n.title}\n\n${n.body}\n`;
}

for (const n of notes) {
  if (n.exportedPath && n.exportedAt) files.set(n.exportedPath.split('\\').pop()!, { name: n.exportedPath.split('\\').pop()!, content: renderMarkdown(n), modified: n.exportedAt, fromOmnihub: true });
}
files.set('REPLY-storage-treemap-zoom.md', {
  name: 'REPLY-storage-treemap-zoom.md',
  fromOmnihub: false,
  modified: Math.floor(NOW - 11 * DAY),
  content: `# Re: Storage treemap zoom

Nice idea. A few notes from the implementation side:

1. **Keep the old frame** in an offscreen canvas while the new layout loads — the zoom then never starts from a blank state.
2. Interpolate in *layout space*, not screen space, so resizing mid-animation stays correct.
3. Use \`requestAnimationFrame\` and cap work at ~1500 rectangles; beyond that fold the long tail.

\`\`\`ts
function lerpRect(a: Rect, b: Rect, t: number): Rect {
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t, w: a.w + (b.w - a.w) * t, h: a.h + (b.h - a.h) * t };
}
\`\`\`

Happy to sketch the squarify part next.
— Claude`,
});
files.set('claude-notes.md', {
  name: 'claude-notes.md',
  fromOmnihub: false,
  modified: Math.floor(NOW - 2 * DAY),
  content: '# Running notes\n\n- Treemap: squarified layout (Bruls, Huizing, van Wijk 2000)\n- Vault: never log secrets, zero buffers after use\n- Phone: pairing window closes after 5 min or 5 wrong PINs',
});

function writeIndex() {
  if (!settings.notes.indexFile) return;
  const exported = notes.filter((n) => n.exportedPath).sort((a, b) => (b.exportedAt ?? 0) - (a.exportedAt ?? 0));
  const body = `# Ideas for Claude\n\n${exported.map((n) => `- [${n.title}](${n.exportedPath!.split('\\').pop()}) — ${new Date((n.exportedAt ?? 0) * 1000).toISOString().slice(0, 10)}`).join('\n')}\n`;
  files.set('INDEX.md', { name: 'INDEX.md', content: body, modified: Math.floor(Date.now() / 1000), fromOmnihub: true });
}
writeIndex();
files.get('INDEX.md')!.modified = Math.floor(NOW - 4 * DAY);

// ---------- commands ----------

const sortNotes = (a: Note, b: Note) => Number(b.pinned) - Number(a.pinned) || b.updated - a.updated;

export function notesList(f: NoteFilter): Note[] {
  const q = (f.query ?? '').trim().toLowerCase();
  let out = notes.filter((n) => (!f.kind || n.kind === f.kind) && (!f.tag || n.tags.includes(f.tag)) && (!q || `${n.title}\n${n.body}\n${n.tags.join(' ')}`.toLowerCase().includes(q)));
  out.sort(sortNotes);
  if (f.limit) out = out.slice(0, f.limit);
  return out.map((n) => ({ ...n, tags: [...n.tags] }));
}

function find(id: string): Note {
  const n = notes.find((x) => x.id === id);
  if (!n) throw new Error('note not found');
  return n;
}

export function noteGet(id: string): Note {
  return { ...find(id) };
}

export function noteSave(input: NoteInput): Note {
  const now = Math.floor(Date.now() / 1000);
  let n: Note;
  if (input.id) {
    n = find(input.id);
    Object.assign(n, { kind: input.kind, title: input.title, body: input.body, tags: [...input.tags], pinned: input.pinned, color: input.color ?? null, updated: now });
  } else {
    n = { id: nid(), kind: input.kind, title: input.title, body: input.body, tags: [...input.tags], pinned: input.pinned, color: input.color ?? null, created: now, updated: now, exportedPath: null, exportedAt: null };
    notes.push(n);
  }
  emit('notes:changed', { id: n.id });
  if (n.kind === 'idea' && settings.notes.autoExportIdeas && settings.notes.claudeFolder) return noteExport(n.id);
  return { ...n, tags: [...n.tags] };
}

export function noteDelete(id: string): void {
  const i = notes.findIndex((x) => x.id === id);
  if (i < 0) throw new Error('note not found');
  notes.splice(i, 1);
  emit('notes:changed', { id, deleted: true });
}

export function noteTags(): [string, number][] {
  const m = new Map<string, number>();
  for (const n of notes) for (const t of n.tags) m.set(t, (m.get(t) ?? 0) + 1);
  return [...m.entries()].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
}

const slugify = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
    .slice(0, 60) || 'untitled';

const replied = new Set<string>();

export function noteExport(id: string): Note {
  const folder = settings.notes.claudeFolder;
  if (!folder) throw new Error('No Claude folder is set. Choose one first (Settings → Notes).');
  const n = find(id);
  const sameFolder = n.exportedPath && n.exportedPath.slice(0, n.exportedPath.lastIndexOf('\\')).toLowerCase() === folder.toLowerCase();
  const name = sameFolder ? n.exportedPath!.split('\\').pop()! : `${new Date(n.created * 1000).toISOString().slice(0, 10)}-${slugify(n.title)}.md`;
  const now = Math.floor(Date.now() / 1000);
  n.exportedPath = `${folder}\\${name}`;
  n.exportedAt = now;
  files.set(name, { name, content: renderMarkdown(n), modified: now, fromOmnihub: true });
  if (settings.notes.sidecarJson) files.set(name.replace(/\.md$/, '.json'), { name: name.replace(/\.md$/, '.json'), content: JSON.stringify(n, null, 2), modified: now, fromOmnihub: true });
  writeIndex();
  emit('notes:exported', { id, path: n.exportedPath });
  emit('notes:folder-changed', { folder });
  // Simulate Claude answering a little later.
  if (!replied.has(id)) {
    replied.add(id);
    setTimeout(() => {
      const reply = `REPLY-${slugify(n.title)}.md`;
      files.set(reply, { name: reply, fromOmnihub: false, modified: Math.floor(Date.now() / 1000), content: `# Re: ${n.title}\n\nThanks — I read the idea. Here's a first plan:\n\n1. Start with the smallest version that proves the interaction.\n2. Measure it on a real drive with ~2M files.\n3. Iterate on the copy once the behaviour feels right.\n\n— Claude` });
      emit('notes:folder-changed', { folder });
    }, 9000);
  }
  return { ...n, tags: [...n.tags] };
}

export function folderFiles(): FolderFile[] {
  const folder = settings.notes.claudeFolder;
  if (!folder) return [];
  return [...files.values()]
    .map((f) => ({ name: f.name, path: `${folder}\\${f.name}`, size: new Blob([f.content]).size, modified: f.modified, isDir: false, fromOmnihub: f.fromOmnihub, preview: f.content.replace(/^---[\s\S]*?---\s*/, '').replace(/[#>*`|]/g, '').replace(/\s+/g, ' ').trim().slice(0, 180) }))
    .sort((a, b) => b.modified - a.modified);
}

export function folderRead(path: string): string {
  const folder = settings.notes.claudeFolder;
  if (!folder || !path.toLowerCase().startsWith(folder.toLowerCase())) throw new Error('That file is outside the Claude folder.');
  const f = files.get(path.split('\\').pop() ?? '');
  if (!f) throw new Error('The system cannot find the file specified. (os error 2)');
  return f.content;
}
