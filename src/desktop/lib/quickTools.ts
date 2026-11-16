// Answers the command palette gives straight away, without leaving it:
// a calculator, unit conversions, Base64, JSON, UUIDs, hashes and Unix
// timestamps. Enter copies the answer. Everything runs locally.

export interface QuickAnswer {
  id: string;
  /** The answer, as shown and as copied. */
  value: string;
  /** What it is ("2 + 2", "5 km in miles"). */
  hint: string;
}

// ---------- calculator: a small parser, never eval ----------

const FUNCS: Record<string, (x: number) => number> = {
  sqrt: Math.sqrt,
  abs: Math.abs,
  round: Math.round,
  floor: Math.floor,
  ceil: Math.ceil,
  ln: Math.log,
  log: Math.log10,
  sin: (x) => Math.sin((x * Math.PI) / 180),
  cos: (x) => Math.cos((x * Math.PI) / 180),
  tan: (x) => Math.tan((x * Math.PI) / 180),
};
const CONSTS: Record<string, number> = { pi: Math.PI, e: Math.E };

/** The value of an arithmetic expression, or null when it isn't one. */
export function calc(input: string): number | null {
  const src = input.toLowerCase().replace(/×/g, '*').replace(/÷/g, '/').replace(/,/g, '').replace(/\s+/g, '');
  if (!src || !/[\d)]/.test(src) || !/^[\d+\-*/%^().a-z]+$/.test(src)) return null;
  let i = 0;
  const peek = () => src[i];
  const eat = (c: string) => (src[i] === c ? (i++, true) : false);

  function primary(): number {
    if (eat('(')) {
      const v = expr();
      if (!eat(')')) throw new Error('(');
      return v;
    }
    const m = /^\d*\.?\d+(e[+-]?\d+)?/.exec(src.slice(i));
    if (m) {
      i += m[0].length;
      return Number(m[0]);
    }
    const w = /^[a-z]+/.exec(src.slice(i));
    if (w) {
      i += w[0].length;
      if (w[0] in CONSTS) return CONSTS[w[0]];
      if (w[0] in FUNCS && peek() === '(') return FUNCS[w[0]](primary());
    }
    throw new Error('bad');
  }
  function unary(): number {
    if (eat('-')) return -unary();
    if (eat('+')) return unary();
    let v = primary();
    if (eat('%')) v /= 100;
    return v;
  }
  function power(): number {
    const b = unary();
    return eat('^') ? b ** power() : b;
  }
  function term(): number {
    let v = power();
    for (;;) {
      if (eat('*')) v *= power();
      else if (eat('/')) v /= power();
      else if (peek() === '(' || /[a-z]/.test(peek() ?? '')) v *= power(); // 2pi, 3(4+1)
      else return v;
    }
  }
  function expr(): number {
    let v = term();
    for (;;) {
      if (eat('+')) v += term();
      else if (eat('-')) v -= term();
      else return v;
    }
  }
  try {
    const v = expr();
    return i === src.length && Number.isFinite(v) ? v : null;
  } catch {
    return null;
  }
}

/** A number as people write it: no float noise, thousands kept plain for copying. */
export function tidy(n: number): string {
  if (Number.isInteger(n) && Math.abs(n) < 1e21) return String(n);
  const s = Number(n.toPrecision(12)).toString();
  return s.includes('e') ? n.toPrecision(10) : s;
}

// ---------- units ----------

type Unit = { kind: string; f: number; o?: number };
const U: Record<string, Unit> = {};
const def = (kind: string, f: number, names: string[], o = 0) => names.forEach((n) => (U[n] = { kind, f, o }));
def('length', 1, ['m', 'meter', 'meters', 'metre', 'metres']);
def('length', 1000, ['km', 'kilometer', 'kilometers', 'kilometre', 'kilometres']);
def('length', 0.01, ['cm', 'centimeter', 'centimeters']);
def('length', 0.001, ['mm', 'millimeter', 'millimeters']);
def('length', 1609.344, ['mi', 'mile', 'miles']);
def('length', 0.3048, ['ft', 'foot', 'feet']);
def('length', 0.0254, ['in', 'inch', 'inches']);
def('length', 0.9144, ['yd', 'yard', 'yards']);
def('mass', 1, ['kg', 'kilo', 'kilos', 'kilogram', 'kilograms']);
def('mass', 0.001, ['g', 'gram', 'grams']);
def('mass', 0.45359237, ['lb', 'lbs', 'pound', 'pounds']);
def('mass', 0.028349523125, ['oz', 'ounce', 'ounces']);
def('mass', 6.35029318, ['st', 'stone']);
def('volume', 1, ['l', 'liter', 'liters', 'litre', 'litres']);
def('volume', 0.001, ['ml', 'milliliter', 'milliliters']);
def('volume', 3.785411784, ['gal', 'gallon', 'gallons']);
def('volume', 0.2365882365, ['cup', 'cups']);
def('volume', 0.0295735295625, ['floz']);
def('speed', 1, ['km/h', 'kmh', 'kph']);
def('speed', 1.609344, ['mph']);
def('speed', 3.6, ['m/s']);
def('speed', 1.852, ['knot', 'knots', 'kn']);
def('data', 1, ['b', 'byte', 'bytes']);
for (const [i, p] of ['k', 'm', 'g', 't', 'p'].entries()) {
  def('data', 1000 ** (i + 1), [`${p}b`]);
  def('data', 1024 ** (i + 1), [`${p}ib`]);
}
def('time', 1, ['s', 'sec', 'secs', 'second', 'seconds']);
def('time', 60, ['min', 'mins', 'minute', 'minutes']);
def('time', 3600, ['h', 'hr', 'hrs', 'hour', 'hours']);
def('time', 86400, ['d', 'day', 'days']);
def('time', 604800, ['wk', 'week', 'weeks']);
// Temperatures: to kelvin = value * f + o.
def('temp', 1, ['k', 'kelvin']);
def('temp', 1, ['c', '°c', 'celsius'], 273.15);
def('temp', 5 / 9, ['f', '°f', 'fahrenheit'], 273.15 - (32 * 5) / 9);

export function convert(input: string): { value: string; hint: string } | null {
  const m = /^(-?[\d.,]+)\s*([a-z°/]+)\s+(?:to|in|as|=)\s+([a-z°/]+)$/i.exec(input.trim());
  if (!m) return null;
  const n = Number(m[1].replace(/,/g, ''));
  const a = U[m[2].toLowerCase()];
  const b = U[m[3].toLowerCase()];
  if (!Number.isFinite(n) || !a || !b || a.kind !== b.kind) return null;
  const base = n * a.f + (a.o ?? 0);
  const out = (base - (b.o ?? 0)) / b.f;
  return { value: `${tidy(Number(out.toPrecision(10)))} ${m[3]}`, hint: `${m[1]} ${m[2]} in ${m[3]}` };
}

// ---------- text tools ----------

function utf8ToBase64(s: string) {
  const bytes = new TextEncoder().encode(s);
  let bin = '';
  bytes.forEach((b) => (bin += String.fromCharCode(b)));
  return btoa(bin);
}

function base64ToUtf8(s: string): string | null {
  try {
    const bin = atob(s.replace(/-/g, '+').replace(/_/g, '/').replace(/\s+/g, ''));
    const text = new TextDecoder('utf-8', { fatal: true }).decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
    // Only readable text is worth showing.
    return /^[\P{C}\n\r\t]*$/u.test(text) ? text : null;
  } catch {
    return null;
  }
}

async function digest(kind: 'SHA-1' | 'SHA-256' | 'SHA-512', text: string) {
  const buf = await crypto.subtle.digest(kind, new TextEncoder().encode(text));
  return [...new Uint8Array(buf)].map((b) => b.toString(16).padStart(2, '0')).join('');
}

function when(ms: number) {
  const d = new Date(ms);
  return `${d.toLocaleString()} (${d.toISOString()})`;
}

/** Everything the palette can answer for `q`, best first. */
export async function quickAnswers(q: string): Promise<QuickAnswer[]> {
  const t = q.trim();
  if (!t) return [];
  const out: QuickAnswer[] = [];
  const low = t.toLowerCase();

  const n = calc(t.replace(/^=/, ''));
  // A bare number is not a sum; "2+2", "sqrt(2)", "15% * 80" are.
  if (n !== null && !/^-?[\d.,]+$/.test(t)) out.push({ id: 'calc', value: tidy(n), hint: `${t} =` });

  const c = convert(t);
  if (c) out.push({ id: 'convert', ...c });

  if (low === 'uuid' || low === 'guid' || low === 'new uuid') out.push({ id: 'uuid', value: crypto.randomUUID(), hint: 'A new random UUID' });
  if (low === 'now' || low === 'timestamp' || low === 'unix time') {
    const s = Math.floor(Date.now() / 1000);
    out.push({ id: 'now', value: String(s), hint: `Unix time now · ${when(s * 1000)}` });
  }
  if (/^\d{10}$/.test(t)) out.push({ id: 'ts', value: when(Number(t) * 1000), hint: `Unix time ${t} (seconds)` });
  if (/^\d{13}$/.test(t)) out.push({ id: 'tsms', value: when(Number(t)), hint: `Unix time ${t} (milliseconds)` });

  const enc = /^(?:base64|b64)\s+(.+)$/is.exec(t);
  if (enc) out.push({ id: 'b64e', value: utf8ToBase64(enc[1]), hint: 'Base64 of the text' });
  const dec = /^(?:unbase64|base64d|b64d|decode base64|from base64)\s+(.+)$/is.exec(t);
  if (dec) {
    const text = base64ToUtf8(dec[1]);
    out.push(text === null ? { id: 'b64d', value: '', hint: 'That isn’t Base64 of readable text' } : { id: 'b64d', value: text, hint: 'Decoded Base64' });
  }

  if (/^[[{]/.test(t)) {
    try {
      const v: unknown = JSON.parse(t);
      out.push({ id: 'json', value: JSON.stringify(v, null, 2), hint: 'Formatted JSON' });
      out.push({ id: 'jsonmin', value: JSON.stringify(v), hint: 'Minified JSON' });
    } catch (e) {
      out.push({ id: 'json', value: '', hint: `Not valid JSON: ${(e as Error).message}` });
    }
  }

  const h = /^(sha1|sha256|sha512|sha-1|sha-256|sha-512)\s+(.+)$/is.exec(t);
  if (h && crypto.subtle) {
    const kind = (h[1].toUpperCase().replace('SHA', 'SHA-').replace('--', '-')) as 'SHA-1' | 'SHA-256' | 'SHA-512';
    out.push({ id: 'hash', value: await digest(kind, h[2]), hint: `${kind} of the text` });
  }
  return out.filter((a) => a.value || a.hint);
}
