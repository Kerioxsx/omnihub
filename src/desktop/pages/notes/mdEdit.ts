// Markdown formatting operations on a textarea selection.

export type MdAction = 'bold' | 'italic' | 'heading' | 'list' | 'checklist' | 'code' | 'link';

export interface EditResult {
  value: string;
  start: number;
  end: number;
}

function wrap(value: string, start: number, end: number, before: string, after = before, placeholder = 'text'): EditResult {
  const sel = value.slice(start, end) || placeholder;
  // Toggle off when the selection is already wrapped.
  if (value.slice(start - before.length, start) === before && value.slice(end, end + after.length) === after) {
    return { value: value.slice(0, start - before.length) + value.slice(start, end) + value.slice(end + after.length), start: start - before.length, end: end - before.length };
  }
  const next = value.slice(0, start) + before + sel + after + value.slice(end);
  return { value: next, start: start + before.length, end: start + before.length + sel.length };
}

function prefixLines(value: string, start: number, end: number, prefix: string, pattern: RegExp): EditResult {
  const lineStart = value.lastIndexOf('\n', start - 1) + 1;
  const lineEnd = value.indexOf('\n', end);
  const stop = lineEnd === -1 ? value.length : lineEnd;
  const block = value.slice(lineStart, stop);
  const lines = block.split('\n');
  const all = lines.every((l) => pattern.test(l));
  const changed = lines.map((l) => (all ? l.replace(pattern, '') : prefix + l.replace(pattern, ''))).join('\n');
  const next = value.slice(0, lineStart) + changed + value.slice(stop);
  return { value: next, start: lineStart, end: lineStart + changed.length };
}

export function applyMd(action: MdAction, value: string, start: number, end: number): EditResult {
  switch (action) {
    case 'bold':
      return wrap(value, start, end, '**');
    case 'italic':
      return wrap(value, start, end, '_');
    case 'code': {
      const sel = value.slice(start, end);
      return sel.includes('\n') ? wrap(value, start, end, '```\n', '\n```', 'code') : wrap(value, start, end, '`', '`', 'code');
    }
    case 'heading':
      return prefixLines(value, start, end, '## ', /^#{1,6}\s+/);
    case 'list':
      return prefixLines(value, start, end, '- ', /^\s*[-*+]\s+(?!\[[ xX]\])/);
    case 'checklist':
      return prefixLines(value, start, end, '- [ ] ', /^\s*[-*+]\s+\[[ xX]\]\s+/);
    case 'link': {
      const sel = value.slice(start, end) || 'link text';
      const next = `${value.slice(0, start)}[${sel}](https://)${value.slice(end)}`;
      const urlStart = start + sel.length + 3;
      return { value: next, start: urlStart, end: urlStart + 8 };
    }
  }
}
