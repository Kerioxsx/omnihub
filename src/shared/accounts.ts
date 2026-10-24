// Is a vault entry the account at a big identity provider itself (its
// password unlocks many others)? Mirrors `vault::is_primary_account`.

const PROVIDERS: [string[], 'Google' | 'Microsoft' | 'Apple' | 'Yahoo' | 'Proton'][] = [
  [['google.com', 'gmail.com', 'googlemail.com'], 'Google'],
  [['microsoft.com', 'microsoftonline.com', 'live.com', 'outlook.com', 'hotmail.com', 'msn.com'], 'Microsoft'],
  [['apple.com', 'icloud.com', 'me.com'], 'Apple'],
  [['yahoo.com'], 'Yahoo'],
  [['proton.me', 'protonmail.com'], 'Proton'],
];

function urlHost(url: string): string | null {
  const u = url.trim();
  if (!u) return null;
  const rest = u.includes('://') ? u.split('://')[1] : u;
  const authority = rest.split(/[/?#]/)[0];
  const host = (authority.includes('@') ? authority.slice(authority.lastIndexOf('@') + 1) : authority).split(':')[0].replace(/\.$/, '').toLowerCase();
  return host || null;
}

function providerOf(domain: string) {
  const d = domain.toLowerCase();
  return PROVIDERS.find(([ds]) => ds.some((p) => d === p || d.endsWith(`.${p}`)))?.[1] ?? null;
}

/** The provider name when the entry is that provider's own account. */
export function primaryProvider(e: { url: string; username: string; email: string }) {
  const host = urlHost(e.url);
  if (host) return providerOf(host);
  for (const id of [e.email, e.username]) {
    const at = id.lastIndexOf('@');
    if (at > 0) {
      const p = providerOf(id.slice(at + 1));
      if (p) return p;
    }
  }
  return null;
}
