// Event bus shared by every mock module (stands in for Tauri events).

type Handler = (payload: unknown) => void;
const listeners = new Map<string, Set<Handler>>();

export function listen(topic: string, handler: Handler): () => void {
  let set = listeners.get(topic);
  if (!set) {
    set = new Set();
    listeners.set(topic, set);
  }
  set.add(handler);
  return () => {
    listeners.get(topic)?.delete(handler);
  };
}

export function emit(topic: string, payload: unknown): void {
  // Deliver asynchronously, like real IPC events.
  queueMicrotask(() => listeners.get(topic)?.forEach((h) => h(payload)));
}

/** Arguments arrive as an untyped record; these read them with narrowing. */
export type Args = Record<string, unknown>;

export function str(args: Args, key: string): string {
  const v = args[key];
  if (typeof v !== 'string') throw new Error(`missing argument "${key}"`);
  return v;
}

export function optStr(args: Args, key: string): string | null {
  const v = args[key];
  return typeof v === 'string' ? v : null;
}

export function num(args: Args, key: string, fallback?: number): number {
  const v = args[key];
  if (typeof v === 'number' && Number.isFinite(v)) return v;
  if (fallback !== undefined) return fallback;
  throw new Error(`missing argument "${key}"`);
}

export function bool(args: Args, key: string, fallback = false): boolean {
  const v = args[key];
  return typeof v === 'boolean' ? v : fallback;
}

export function obj<T>(args: Args, key: string): T {
  const v = args[key];
  if (typeof v !== 'object' || v === null) throw new Error(`missing argument "${key}"`);
  return v as T;
}

export function strList(args: Args, key: string): string[] {
  const v = args[key];
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
}
