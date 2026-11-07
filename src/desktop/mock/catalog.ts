// Demo catalogue for Aurora's search (made-up artists, generated covers).

import type { CatAlbum, CatAlbumPage, CatArtist, CatArtistPage, CatResults, CatTrack, CatUpNext, MusicSource, PlayItem } from '@shared/types';
import { svgUrl } from './art';

const PALETTES: [string, string][] = [
  ['#7c3aed', '#22d3ee'],
  ['#f43f5e', '#f59e0b'],
  ['#0ea5e9', '#a3e635'],
  ['#111827', '#e879f9'],
  ['#14b8a6', '#6366f1'],
  ['#fb7185', '#1e3a8a'],
];

function cover(seed: string, label: string): string {
  let h = 0;
  for (const c of seed) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  const [a, b] = PALETTES[h % PALETTES.length];
  const r = 120 + (h % 160);
  return svgUrl(
    `<svg xmlns="http://www.w3.org/2000/svg" width="500" height="500" viewBox="0 0 500 500"><defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="${a}"/><stop offset="1" stop-color="${b}"/></linearGradient></defs><rect width="500" height="500" fill="url(#g)"/><circle cx="${150 + (h % 200)}" cy="${140 + ((h >> 3) % 220)}" r="${r}" fill="#fff" opacity=".14"/><text x="34" y="460" font-family="Segoe UI, Arial" font-size="40" font-weight="800" fill="#fff" opacity=".9">${label.replace(/[<&>]/g, '')}</text></svg>`,
  );
}

const artists: CatArtist[] = [
  { id: '1', name: 'Nova Lane', picture: cover('nova-person', 'NOVA'), fans: 2_480_000, albums: 6 },
  { id: '2', name: 'The Midnight Arcade', picture: cover('arcade-person', 'ARCADE'), fans: 910_000, albums: 4 },
  { id: '3', name: 'Kairo', picture: cover('kairo-person', 'KAIRO'), fans: 5_120_000, albums: 9 },
  { id: '4', name: 'Midnight Atlas', picture: cover('atlas-person', 'ATLAS'), fans: 1_340_000, albums: 3 },
];

function album(id: string, title: string, a: CatArtist, year: string, kind: string, n: number): CatAlbum {
  return { id, title, artist: a.name, artistId: a.id, cover: cover(`${title}-${a.name}`, title.toUpperCase().slice(0, 12)), year, kind, tracks: n };
}

const albums: CatAlbum[] = [
  album('10', 'Neon Hearts', artists[0], '2026', 'album', 6),
  album('11', 'Afterglow', artists[0], '2025', 'single', 1),
  album('12', 'City of Glass', artists[0], '2023', 'album', 5),
  album('13', 'Lowlight EP', artists[0], '2022', 'ep', 4),
  album('20', 'Pixel Skies', artists[1], '2024', 'album', 5),
  album('30', 'Northside', artists[2], '2026', 'album', 6),
  album('31', 'Gravity', artists[2], '2024', 'single', 1),
  album('40', 'Open Roads', artists[3], '2026', 'album', 6),
  album('41', 'Coastline', artists[3], '2024', 'album', 4),
];

const TITLES: Record<string, string[]> = {
  '10': ['Neon Hearts', 'Midnight Drive', 'Glass Rooms', 'Afterhours', 'Satellites', 'Love Me Loud'],
  '11': ['Afterglow'],
  '12': ['City of Glass', 'Paper Planes', 'Static', 'Gold Rush', 'Echoes'],
  '13': ['Lowlight', 'Slow Motion', 'Faded', 'Daydream'],
  '20': ['Pixel Skies', 'Insert Coin', 'Game Over', 'High Score', 'Respawn'],
  '30': ['Northside', 'Noticed', 'Blue Faygo', 'Kamikaze', 'Pull Up', 'Jetski'],
  '31': ['Gravity'],
  '40': ['Daylight Drive', 'Night Loop', 'Static Bloom', 'Unwritten Demo', 'Overpass', 'Last Exit'],
  '41': ['Coastline', 'Salt Air', 'Lighthouse', 'Undertow'],
};

function tracksOf(al: CatAlbum): CatTrack[] {
  return (TITLES[al.id] ?? []).map((t, i) => ({ id: `${al.id}${i}`, title: t, artist: al.artist, artistId: al.artistId, album: al.title, albumId: al.id, cover: al.cover, durationMs: 150_000 + ((i * 37_000) % 90_000), explicit: i % 3 === 1 }));
}

const allTracks = albums.flatMap(tracksOf);
const BIOS: Record<string, string> = {
  '1': 'Nova Lane is a singer and producer known for glossy synth-pop and late-night drives. (Demo artist.)',
  '2': 'The Midnight Arcade is a duo making retro-gaming inspired electronic music. (Demo artist.)',
  '4': 'Midnight Atlas is an electronic band known for warm, driving synth anthems. (Demo artist.)',
  '3': 'Kairo is a rapper and songwriter whose melodic hooks topped streaming charts. (Demo artist.)',
};

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const has = (s: string, q: string) => s.toLowerCase().includes(q);

export async function search(query: string): Promise<CatResults> {
  await sleep(250);
  const q = query.trim().toLowerCase();
  if (!q) return { top: null, tracks: [], albums: [], artists: [] };
  const ar = artists.filter((a) => has(a.name, q));
  const tr = allTracks.filter((t) => has(t.title, q) || has(t.artist, q)).slice(0, 15);
  const al = albums.filter((a) => has(a.title, q) || has(a.artist, q));
  const exact = ar.find((a) => a.name.toLowerCase() === q);
  return { top: exact ? { kind: 'artist', item: exact } : tr[0] ? { kind: 'track', item: tr[0] } : null, tracks: tr, albums: al, artists: ar };
}

export async function artist(id: string): Promise<CatArtistPage> {
  await sleep(200);
  const a = artists.find((x) => x.id === id);
  if (!a) throw 'That artist isn’t in the catalogue.';
  return { artist: a, bio: BIOS[id] ?? null, top: allTracks.filter((t) => t.artistId === id).slice(0, 10), albums: albums.filter((x) => x.artistId === id) };
}

export async function albumPage(id: string): Promise<CatAlbumPage> {
  await sleep(150);
  const al = albums.find((x) => x.id === id);
  if (!al) throw 'That album isn’t in the catalogue.';
  return { album: al, tracks: tracksOf(al) };
}

export async function upNext(title: string, artistName: string): Promise<CatUpNext> {
  await sleep(200);
  const t = allTracks.find((x) => x.title.toLowerCase() === title.toLowerCase() && x.artist.toLowerCase() === artistName.toLowerCase());
  if (!t) return { track: null, album: null, more: [], artist: null };
  const al = albums.find((x) => x.id === t.albumId)!;
  const a = artists.find((x) => x.id === t.artistId)!;
  return { track: t, album: { album: al, tracks: tracksOf(al) }, more: allTracks.filter((x) => x.artistId === a.id && x.albumId !== al.id).slice(0, 8), artist: a };
}

export function play(_item: PlayItem, source: MusicSource): MusicSource {
  return source === 'auto' ? 'spotify' : source;
}
