// In-memory file tree used by the storage mock. Mirrors the semantics of
// crates/omnihub-core/src/storage/tree.rs: pre-order ids, directory sizes
// aggregated bottom-up, treemap folding, search with globs.

import type { ExtensionStat, NodeView, SearchQuery, SortKey, TreemapItem } from '@shared/types';

export const FLAG_HIDDEN = 1;
export const FLAG_SYSTEM = 2;
export const FLAG_REPARSE = 4;
export const FLAG_CLOUD = 8;
export const FLAG_VIRTUAL = 16;

export const FOLDED_ID = 4294967295;

export interface FNode {
  id: number;
  parent: number;
  name: string;
  isDir: boolean;
  size: number;
  alloc: number;
  modified: number;
  flags: number;
  children: number[];
  files: number;
  dirs: number;
  deleted: boolean;
}

export function extensionOf(name: string): string | null {
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) return null;
  const ext = name.slice(dot + 1);
  if (ext.length > 16 || ext.includes(' ')) return null;
  return ext.toLowerCase();
}

export class NameMatcher {
  private readonly needle: string;
  private readonly glob: RegExp | null;

  constructor(text: string) {
    this.needle = text.trim().toLowerCase();
    if (/[*?]/.test(this.needle)) {
      const re = this.needle.replace(/[.+^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\?/g, '.');
      this.glob = new RegExp(`^${re}$`);
    } else {
      this.glob = null;
    }
  }

  matches(name: string): boolean {
    if (!this.needle) return true;
    const lower = name.toLowerCase();
    return this.glob ? this.glob.test(lower) : lower.includes(this.needle);
  }
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

export class FakeTree {
  readonly nodes: FNode[] = [];
  readonly clusterSize = 4096;

  constructor(rootName: string, modified: number) {
    this.nodes.push({ id: 0, parent: -1, name: rootName, isDir: true, size: 0, alloc: 0, modified, flags: 0, children: [], files: 0, dirs: 0, deleted: false });
  }

  add(parent: number, name: string, isDir: boolean, size: number, modified: number, flags = 0): number {
    const id = this.nodes.length;
    const alloc = isDir ? 0 : size < 700 ? 0 : Math.ceil(size / this.clusterSize) * this.clusterSize;
    this.nodes.push({ id, parent, name, isDir, size: isDir ? 0 : size, alloc, modified, flags, children: [], files: 0, dirs: 0, deleted: false });
    this.nodes[parent].children.push(id);
    return id;
  }

  /** Aggregate sizes and counts bottom-up (children always have larger ids). */
  finish(): void {
    for (const n of this.nodes) {
      if (n.isDir) {
        n.size = 0;
        n.alloc = 0;
        n.files = 0;
        n.dirs = 0;
      } else {
        n.files = 1;
        n.dirs = 0;
      }
    }
    for (let i = this.nodes.length - 1; i > 0; i--) {
      const n = this.nodes[i];
      if (n.deleted) continue;
      const p = this.nodes[n.parent];
      p.size += n.size;
      p.alloc += n.alloc;
      p.files += n.files;
      p.dirs += n.dirs + (n.isDir ? 1 : 0);
      if (n.isDir && n.modified > p.modified) p.modified = n.modified;
    }
  }

  node(id: number): FNode {
    const n = this.nodes[id];
    if (!n) throw new Error(`node ${id} not found`);
    return n;
  }

  liveChildren(id: number): FNode[] {
    return this.node(id).children.map((c) => this.nodes[c]).filter((c) => !c.deleted);
  }

  path(id: number): string {
    const parts: string[] = [];
    let cur: FNode | undefined = this.node(id);
    while (cur && cur.parent >= 0) {
      parts.push(cur.name);
      cur = this.nodes[cur.parent];
    }
    const root = this.nodes[0].name.replace(/\\$/, '');
    return parts.length ? `${root}\\${parts.reverse().join('\\')}` : this.nodes[0].name;
  }

  findPath(path: string): number | null {
    const root = this.nodes[0].name.replace(/\\$/, '').toLowerCase();
    const norm = path.replace(/\//g, '\\').replace(/\\+$/, '').toLowerCase();
    if (norm === root) return 0;
    if (!norm.startsWith(root + '\\')) return null;
    let cur = 0;
    for (const comp of norm.slice(root.length + 1).split('\\').filter(Boolean)) {
      const next = this.node(cur).children.find((c) => !this.nodes[c].deleted && this.nodes[c].name.toLowerCase() === comp);
      if (next === undefined) return null;
      cur = next;
    }
    return cur;
  }

  /** Every live node id in the subtree (pre-order). */
  subtree(id: number): number[] {
    const out: number[] = [];
    const stack = [id];
    while (stack.length) {
      const cur = stack.pop()!;
      const n = this.nodes[cur];
      if (n.deleted) continue;
      out.push(cur);
      for (let i = n.children.length - 1; i >= 0; i--) stack.push(n.children[i]);
    }
    return out;
  }

  contains(ancestor: number, id: number): boolean {
    let cur: number = id;
    while (cur >= 0) {
      if (cur === ancestor) return true;
      cur = this.nodes[cur].parent;
    }
    return false;
  }

  view(id: number): NodeView {
    const n = this.node(id);
    const parentSize = n.parent >= 0 ? this.nodes[n.parent].size : n.size;
    return {
      id: n.id,
      name: n.name,
      isDir: n.isDir,
      size: n.size,
      alloc: n.alloc,
      files: n.files,
      dirs: n.dirs,
      modified: n.modified,
      hidden: (n.flags & FLAG_HIDDEN) !== 0,
      system: (n.flags & FLAG_SYSTEM) !== 0,
      reparse: (n.flags & FLAG_REPARSE) !== 0,
      cloud: (n.flags & FLAG_CLOUD) !== 0,
      isVirtual: (n.flags & FLAG_VIRTUAL) !== 0,
      children: n.children.filter((c) => !this.nodes[c].deleted).length,
      fraction: parentSize === 0 ? 0 : n.size / parentSize,
    };
  }

  compare(a: FNode, b: FNode, sort: SortKey): number {
    switch (sort) {
      case 'name':
        return collator.compare(a.name, b.name);
      case 'modified':
        return b.modified - a.modified;
      case 'files':
        return b.files - a.files || b.size - a.size;
      case 'alloc':
        return b.alloc - a.alloc;
      default:
        return b.size - a.size;
    }
  }

  children(id: number, sort: SortKey, descending: boolean, offset: number, limit: number): { items: NodeView[]; total: number } {
    const kids = this.liveChildren(id);
    kids.sort((a, b) => this.compare(a, b, sort));
    // compare() is "natural" order (largest/newest first, names A→Z).
    const natural = sort === 'name' ? !descending : descending;
    if (!natural) kids.reverse();
    return { items: kids.slice(offset, offset + limit).map((k) => this.view(k.id)), total: kids.length };
  }

  breadcrumbs(id: number, rootId = 0): { id: number; name: string }[] {
    const out: { id: number; name: string }[] = [];
    let cur = id;
    while (cur >= 0) {
      const n = this.nodes[cur];
      out.push({ id: n.id, name: n.name });
      if (cur === rootId) break;
      cur = n.parent;
    }
    return out.reverse();
  }

  /**
   * Size-ordered nested items for a treemap, at most `maxItems` in total.
   * The budget is spent breadth-first so every level stays complete before
   * deeper levels are filled (see the report: the Rust version is depth-first).
   */
  treemap(id: number, depth: number, maxItems: number): TreemapItem {
    const mk = (n: FNode): TreemapItem => ({ id: n.id, name: n.name, size: n.size, isDir: n.isDir, ext: n.isDir ? null : extensionOf(n.name), folded: 0, children: [] });
    const root = mk(this.node(id));
    let budget = Math.max(1, maxItems);
    let frontier: [TreemapItem, number][] = [[root, depth]];
    while (frontier.length) {
      const next: [TreemapItem, number][] = [];
      for (const [item, d] of frontier) {
        if (!item.isDir || d === 0 || item.size === 0) continue;
        const kids = this.liveChildren(item.id).sort((a, b) => b.size - a.size);
        const min = Math.max(1, Math.floor(item.size / 1000));
        let foldedSize = 0;
        let folded = 0;
        for (const c of kids) {
          if (budget === 0 || c.size < min) {
            foldedSize += c.size;
            folded++;
            continue;
          }
          budget--;
          const child = mk(c);
          item.children.push(child);
          next.push([child, d - 1]);
        }
        if (folded > 0 && foldedSize > 0) {
          item.children.push({ id: FOLDED_ID, name: `${folded} smaller items`, size: foldedSize, isDir: false, ext: null, folded, children: [] });
        }
      }
      frontier = next;
    }
    return root;
  }

  filesUnder(id: number): FNode[] {
    return this.subtree(id)
      .map((i) => this.nodes[i])
      .filter((n) => !n.isDir);
  }

  topFiles(id: number, n: number): NodeView[] {
    return this.filesUnder(id)
      .sort((a, b) => b.size - a.size)
      .slice(0, n)
      .map((f) => this.view(f.id));
  }

  extensions(id: number): ExtensionStat[] {
    const map = new Map<string, ExtensionStat>();
    for (const f of this.filesUnder(id)) {
      const ext = extensionOf(f.name) ?? '';
      let e = map.get(ext);
      if (!e) {
        e = { ext, count: 0, size: 0, alloc: 0 };
        map.set(ext, e);
      }
      e.count++;
      e.size += f.size;
      e.alloc += f.alloc;
    }
    return [...map.values()].sort((a, b) => b.size - a.size || a.ext.localeCompare(b.ext));
  }

  search(q: SearchQuery): { items: NodeView[]; total: number } {
    const under = q.under != null && q.under < this.nodes.length ? q.under : 0;
    const matcher = new NameMatcher(q.text);
    const exts = (q.extensions ?? []).map((e) => e.replace(/^\./, '').toLowerCase()).filter(Boolean);
    const hits: FNode[] = [];
    for (const i of this.subtree(under)) {
      const n = this.nodes[i];
      if (n.parent < 0 || (n.flags & FLAG_VIRTUAL) !== 0) continue;
      if ((q.filesOnly && n.isDir) || (q.dirsOnly && !n.isDir)) continue;
      if ((q.minSize != null && n.size < q.minSize) || (q.maxSize != null && n.size > q.maxSize)) continue;
      if ((q.modifiedBefore != null && n.modified >= q.modifiedBefore) || (q.modifiedAfter != null && n.modified <= q.modifiedAfter)) continue;
      if (exts.length) {
        const e = extensionOf(n.name);
        if (!e || !exts.includes(e)) continue;
      }
      if (matcher.matches(n.name)) hits.push(n);
    }
    const sort = q.sort ?? 'size';
    hits.sort((a, b) => this.compare(a, b, sort));
    const limit = Math.min(q.limit ?? 500, hits.length);
    return { items: hits.slice(0, limit).map((h) => this.view(h.id)), total: hits.length };
  }

  /** Mark a subtree deleted and subtract it from its ancestors. */
  markDeleted(id: number): void {
    const n = this.node(id);
    if (n.deleted || n.parent < 0) return;
    for (const i of this.subtree(id)) this.nodes[i].deleted = true;
    let p = n.parent;
    while (p >= 0) {
      const parent = this.nodes[p];
      parent.size = Math.max(0, parent.size - n.size);
      parent.alloc = Math.max(0, parent.alloc - n.alloc);
      parent.files = Math.max(0, parent.files - n.files);
      parent.dirs = Math.max(0, parent.dirs - n.dirs - (n.isDir ? 1 : 0));
      p = parent.parent;
    }
  }

  /** Resolve a pattern with `*` / `?` components (like cleanup.rs resolve_pattern). */
  resolvePattern(pattern: string): number[] {
    const root = this.nodes[0].name.replace(/\\$/, '').toLowerCase();
    const norm = pattern.replace(/\//g, '\\').toLowerCase();
    if (!norm.startsWith(root)) return [];
    const comps = norm.slice(root.length).split('\\').filter(Boolean);
    let frontier = [0];
    for (const comp of comps) {
      const m = new NameMatcher(comp);
      const glob = /[*?]/.test(comp);
      const next: number[] = [];
      for (const f of frontier) {
        for (const c of this.liveChildren(f)) {
          if (glob ? m.matches(c.name) : c.name.toLowerCase() === comp) next.push(c.id);
        }
      }
      if (!next.length) return [];
      frontier = next;
    }
    return frontier.length === 1 && frontier[0] === 0 ? [] : frontier;
  }
}
