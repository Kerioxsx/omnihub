// Pictures for the browser demo's media library (the app shows the user's own).

const ART: Record<string, [string, string, string]> = {
  aurora: ['#7c3aed', '#00d4ff', '#ff2bd6'],
  ember: ['#ff3b00', '#ffb703', '#2a0a00'],
};

export function demoArt(id: string): string {
  const [a, b, c] = ART[id.includes('ember') ? 'ember' : 'aurora'];
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="640" viewBox="0 0 640 640"><defs><radialGradient id="g" cx="35%" cy="30%" r="80%"><stop offset="0" stop-color="${b}"/><stop offset=".55" stop-color="${a}"/><stop offset="1" stop-color="#05050a"/></radialGradient><linearGradient id="l" x1="0" y1="1" x2="1" y2="0"><stop offset="0" stop-color="${c}" stop-opacity=".0"/><stop offset="1" stop-color="${c}" stop-opacity=".75"/></linearGradient></defs><rect width="640" height="640" fill="url(#g)"/><circle cx="420" cy="420" r="240" fill="url(#l)"/><text x="320" y="350" font-family="Segoe UI, Arial" font-size="92" font-weight="800" fill="#fff" text-anchor="middle" opacity=".92">OMNIHUB</text></svg>`;
  return `data:image/svg+xml;base64,${btoa(svg)}`;
}
