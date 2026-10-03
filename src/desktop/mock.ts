// In-browser mock of the Rust backend (placeholder; see api.ts).
type Handler = (payload: unknown) => void;
const listeners = new Map<string, Set<Handler>>();

export async function mockInvoke(cmd: string, _args: Record<string, unknown>): Promise<unknown> {
  throw new Error(`mock: ${cmd} not implemented`);
}

export function mockListen(topic: string, handler: Handler): () => void {
  if (!listeners.has(topic)) listeners.set(topic, new Set());
  listeners.get(topic)!.add(handler);
  return () => listeners.get(topic)?.delete(handler);
}

export function mockEmit(topic: string, payload: unknown) {
  listeners.get(topic)?.forEach((h) => h(payload));
}
