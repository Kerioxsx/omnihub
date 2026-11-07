// Karaoke Studio: sing along and learn songs in any language. Built on
// Aurora's pieces (Now Playing, its lyrics sources and timing, its word
// fill) with its own state, so the music player doesn't change.
//
// - First visit asks your first language; pronunciation is written for it.
// - Songs: what the PC is playing (Spotify, Apple Music, YouTube…) or a song
//   file from this PC, whose lyrics come from an .lrc beside it, your lyrics
//   folder, LRCLIB or NetEase, or an .lrc you pick.
// - Follow the song: the music keeps its time; the microphone measures your
//   pitch, steadiness, entrances and range.
// - Follow my voice (song files only): the music keeps pace with you,
//   waits when you stop and picks up where you left off.
// The microphone is analysed on this PC only and never recorded; only the
// summary numbers of a practice are saved, and they can be deleted.

import type { KaraokeData, KaraokeLyrics as LyricsData, KaraokeSession, KaraokeTrack, LyricLine } from '@shared/types';
import { FileAudio, FileText, Gauge, Headphones, History, Languages, ListMusic, Mic, MicOff, Pause, Play, Repeat, RotateCcw, SkipForward, Square, Trash, Waves } from 'lucide-react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, errorText } from '../../../api';
import { lineEnd, phaseAt } from '../../../components/aurora/AuroraLyrics';
import { Button, IconButton } from '../../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../../components/ui/Card';
import { Segmented, Select, Switch } from '../../../components/ui/Form';
import { Callout, EmptyState } from '../../../components/ui/States';
import { cx } from '../../../lib/cx';
import { useStoredState } from '../../../lib/hooks';
import { Follower, type FollowState, followLines } from '../../../lib/karaoke/follow';
import { languageOf } from '../../../lib/karaoke/languages';
import { Coach, type CoachSummary, centsOff, listMics, Mic as MicInput, type MicError, type MicInput as MicDevice, micErrorText, noteName, type VoiceFrame } from '../../../lib/karaoke/pitch';
import { detectLanguage, guideNote, readerOf, type SongLanguage } from '../../../lib/karaoke/pron';
import { estimateWords, shiftLines } from '../../../lib/karaoke/timing';
import { fmtTime, useNowPlaying } from '../../../lib/nowPlaying';
import { confirm } from '../../../state/dialogs';
import { toast } from '../../../state/toasts';
import { KaraokeLyrics } from './KaraokeLyrics';
import { LanguageSetup } from './LanguageSetup';

type Source = 'now' | 'file';
type Mode = 'song' | 'voice';

const SONG_LANGS: SongLanguage[] = [
  { code: 'ru', name: 'Russian', script: 'cyrillic' },
  { code: 'uk', name: 'Ukrainian', script: 'cyrillic' },
  { code: 'ja', name: 'Japanese', script: 'kana' },
  { code: 'ko', name: 'Korean', script: 'hangul' },
  { code: 'zh', name: 'Chinese', script: 'han' },
  { code: 'el', name: 'Greek', script: 'greek' },
  { code: 'es', name: 'Spanish', script: 'latin' },
  { code: 'en', name: 'English', script: 'latin' },
  { code: 'pt', name: 'Portuguese', script: 'latin' },
  { code: 'fr', name: 'French', script: 'latin' },
  { code: 'de', name: 'German', script: 'latin' },
  { code: 'it', name: 'Italian', script: 'latin' },
  { code: 'ar', name: 'Arabic', script: 'arabic' },
  { code: 'he', name: 'Hebrew', script: 'hebrew' },
  { code: 'hi', name: 'Hindi', script: 'devanagari' },
  { code: 'th', name: 'Thai', script: 'thai' },
];

const SOLFEGE_READERS = new Set(['es', 'pt', 'fr', 'it']);

/** A held-note wobble in words people know. */
const centsText = (c: number) => `${c} cents (a semitone is 100)`;

// ---------- the live voice meter ----------

function VoiceMeter({ frame, solfege }: { frame: React.MutableRefObject<VoiceFrame | null>; solfege: boolean }) {
  const [f, setF] = useState<VoiceFrame | null>(null);
  useEffect(() => {
    const id = window.setInterval(() => setF(frame.current), 90);
    return () => window.clearInterval(id);
  }, [frame]);
  const level = f ? Math.max(0, Math.min(1, (f.db + 60) / 50)) : 0;
  const cents = f?.midi != null ? centsOff(f.midi) : null;
  return (
    <div className="flex items-center gap-4">
      <div className="w-[76px] text-center">
        <div className={cx('font-display text-[28px] font-bold tabular-nums leading-none', f?.voiced && f.midi != null ? 'text-fg' : 'text-faint')}>{f?.voiced && f.midi != null ? noteName(f.midi, solfege) : '—'}</div>
        <div className="mt-1 text-[11px] text-faint">{f?.voiced ? (f.midi != null ? `${f.hz?.toFixed(0)} Hz` : 'no clear pitch') : 'quiet'}</div>
      </div>
      <div className="min-w-0 flex-1 space-y-2">
        <div className="relative h-2 rounded-full bg-surface-3" title="How far from the nearest note (centre = right on it)">
          <span className="absolute left-1/2 top-[-3px] h-[14px] w-px bg-line-strong" aria-hidden />
          {cents !== null && f?.voiced && <span className={cx('absolute top-[-2px] h-3 w-3 -translate-x-1/2 rounded-full transition-[left] duration-75', Math.abs(cents) <= 15 ? 'bg-good' : Math.abs(cents) <= 30 ? 'bg-warn' : 'bg-bad')} style={{ left: `${50 + cents}%` }} />}
        </div>
        <div className="flex items-center gap-2">
          <span className={cx('h-2 w-2 shrink-0 rounded-full', f?.voiced ? 'bg-good' : 'bg-surface-3')} aria-hidden />
          <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-surface-3">
            <div className="accent-gradient h-full transition-[width] duration-75" style={{ width: `${level * 100}%` }} />
          </div>
        </div>
      </div>
    </div>
  );
}

// ---------- the summary of one practice ----------

function SummaryCard({ s, retry, onPractice, onClose, solfege, mode }: { s: CoachSummary; retry: { i: number; text: string }[]; onPractice: (i: number) => void; onClose: () => void; solfege: boolean; mode: Mode }) {
  return (
    <Card className="p-5">
      <CardHeader icon={Gauge} title="How that went" subtitle={`${fmtTime(s.seconds * 1000)} of practice, ${s.lines} ${s.lines === 1 ? 'line' : 'lines'} sung. Only what was really measured.`} actions={<Button size="sm" variant="ghost" onClick={onClose}>Close</Button>} />
      <div className="mt-4 grid gap-3 sm:grid-cols-3">
        <div className="rounded-xl border border-line bg-surface-2/50 p-3">
          <div className="text-[11.5px] uppercase tracking-wide text-faint">Held notes</div>
          <div className="mt-1 text-[14px] text-fg">{s.steadinessCents !== null ? `Wobble ${centsText(s.steadinessCents)}` : 'Not enough long notes to measure'}</div>
          <div className="mt-1 text-[11.5px] text-faint">Lower is steadier.</div>
        </div>
        <div className="rounded-xl border border-line bg-surface-2/50 p-3">
          <div className="text-[11.5px] uppercase tracking-wide text-faint">Entrances</div>
          <div className="mt-1 text-[14px] text-fg">
            {mode === 'voice' ? 'Not timed: the song followed you' : s.timingMs !== null ? (Math.abs(s.timingMs) < 60 ? 'Right on the lyrics’ timing' : `${Math.abs(s.timingMs)} ms ${s.timingMs < 0 ? 'early' : 'late'} on average`) : 'Not enough line starts to measure'}
          </div>
          <div className="mt-1 text-[11.5px] text-faint">Against the lyrics’ timing{mode === 'song' ? ' (approximate if the timing is)' : ''}.</div>
        </div>
        <div className="rounded-xl border border-line bg-surface-2/50 p-3">
          <div className="text-[11.5px] uppercase tracking-wide text-faint">Your range today</div>
          <div className="mt-1 text-[14px] text-fg">{s.lowNote !== null && s.highNote !== null ? `${noteName(s.lowNote, solfege)} – ${noteName(s.highNote, solfege)}` : 'Sing a little longer to see it'}</div>
          <div className="mt-1 text-[11.5px] text-faint">What you sang comfortably; no need to push it.</div>
        </div>
      </div>
      <p className="mt-3 text-[12px] text-faint">Not scored: pitch against the original melody (this song has no reference melody here) and pronunciation (that needs speech recognition OmniHub doesn’t run yet).</p>
      {retry.length > 0 && (
        <div className="mt-4">
          <div className="mb-1.5 text-[12.5px] font-medium text-dim">Lines to try again</div>
          {retry.map((r) => (
            <div key={r.i} className="flex items-center justify-between gap-3 border-t border-line py-2 first:border-t-0">
              <span className="min-w-0 truncate text-[13.5px] text-fg">{r.text}</span>
              <Button size="xs" icon={Repeat} onClick={() => onPractice(r.i)}>
                Practice
              </Button>
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

function HistoryCard({ sessions, solfege, onClear }: { sessions: KaraokeSession[]; solfege: boolean; onClear: () => void }) {
  return (
    <Card className="p-5">
      <CardHeader
        icon={History}
        title="Practice history"
        subtitle="Summaries only. Nothing you sing is recorded."
        actions={
          sessions.length > 0 && (
            <Button size="sm" variant="ghost" icon={Trash} onClick={onClear}>
              Delete history
            </Button>
          )
        }
      />
      {sessions.length === 0 ? (
        <p className="mt-3 text-[13px] text-faint">Your practices will show here.</p>
      ) : (
        <div className="mt-3 max-h-[280px] overflow-y-auto">
          {sessions.slice(0, 50).map((s) => (
            <div key={s.at} className="border-t border-line py-2.5 first:border-t-0">
              <div className="flex items-baseline justify-between gap-3">
                <span className="min-w-0 truncate text-[13.5px] font-medium text-fg">
                  {s.title || 'A song'}
                  {s.artist && <span className="font-normal text-dim"> · {s.artist}</span>}
                </span>
                <span className="shrink-0 text-[11.5px] text-faint">{new Date(s.at).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</span>
              </div>
              <div className="mt-0.5 flex flex-wrap gap-x-3 text-[12px] text-dim">
                <span>{s.mode === 'voice' ? 'Followed my voice' : 'Followed the song'}</span>
                <span>{fmtTime(s.seconds * 1000)}</span>
                {s.steadinessCents != null && <span>wobble {s.steadinessCents}¢</span>}
                {s.timingMs != null && <span>{Math.abs(s.timingMs) < 60 ? 'on time' : `${Math.abs(s.timingMs)} ms ${s.timingMs < 0 ? 'early' : 'late'}`}</span>}
                {s.lowNote != null && s.highNote != null && (
                  <span>
                    {noteName(s.lowNote, solfege)}–{noteName(s.highNote, solfege)}
                  </span>
                )}
              </div>
              {s.retry.length > 0 && <div className="mt-0.5 truncate text-[11.5px] text-faint">Try again: {s.retry.join(' · ')}</div>}
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}

// ---------- the studio ----------

export function KaraokeStudio() {
  const [data, setData] = useState<KaraokeData | null>(null);
  const [setup, setSetup] = useState(false);
  const [source, setSource] = useStoredState<Source>('karaoke.source', 'now');
  const [mode, setMode] = useStoredState<Mode>('karaoke.mode', 'song');
  const [show, setShow] = useStoredState('karaoke.show', { say: true, syllables: true });
  const [micId, setMicId] = useStoredState<string>('karaoke.mic', '');
  const [songLang, setSongLang] = useState('auto');
  const [offset, setOffset] = useState(0);
  const [speed, setSpeed] = useState(1);
  const [loop, setLoop] = useState<number | null>(null);

  useEffect(() => {
    void api.karaoke.get().then(
      (d) => {
        setData(d);
        if (!d.profile) setSetup(true);
      },
      () => setData({ profile: null, sessions: [] }),
    );
  }, []);

  const lang = languageOf(data?.profile?.lang);
  const reader = readerOf(lang?.code ?? '');
  const readerName = lang?.name.split(' (')[0] ?? 'your language';
  const solfege = SOLFEGE_READERS.has(reader);

  // ----- what's playing -----
  const np = useNowPlaying();
  const audio = useRef<HTMLAudioElement>(null);
  const [track, setTrack] = useState<KaraokeTrack | null>(null);
  const [fileLyrics, setFileLyrics] = useState<{ status: 'idle' | 'loading' | 'ready' | 'none' | 'error'; lyrics?: LyricsData; error?: string }>({ status: 'idle' });
  const [ownLyrics, setOwnLyrics] = useState<{ key: string; lyrics: LyricsData } | null>(null);
  const [playing, setPlaying] = useState(false);
  const wantPlay = useRef(false);
  const [duration, setDuration] = useState(0);

  const fileTime = useCallback(() => (audio.current?.currentTime ?? 0) * 1000, []);
  const time = source === 'file' ? fileTime : np.position;
  const songKey = source === 'file' ? (track?.path ?? '') : (np.state?.key ?? '');
  const title = source === 'file' ? (track?.title ?? '') : (np.state?.title ?? '');
  const artist = source === 'file' ? (track?.artist ?? '') : (np.state?.artist ?? '');

  const raw: LyricLine[] = useMemo(() => {
    if (ownLyrics && ownLyrics.key === songKey) return ownLyrics.lyrics.lines;
    if (source === 'file') return fileLyrics.lyrics?.lines ?? [];
    return np.lines;
  }, [ownLyrics, songKey, source, fileLyrics, np.lines]);
  const lines = useMemo(() => shiftLines(estimateWords(raw), offset), [raw, offset]);
  const estimated = lines.some((l) => l.estimated);
  const lyricSource = ownLyrics && ownLyrics.key === songKey ? ownLyrics.lyrics.source : source === 'file' ? fileLyrics.lyrics?.source : np.lyrics?.status === 'ready' ? np.lyrics.lyrics.source : undefined;

  const detected = useMemo(() => detectLanguage(raw.map((l) => l.text).join(' ')), [raw]);
  const song = songLang === 'auto' ? detected : (SONG_LANGS.find((l) => l.code === songLang) ?? detected);

  // A new song: start fresh.
  useEffect(() => {
    setLoop(null);
    setOffset(0);
    setSongLang('auto');
  }, [songKey]);

  // ----- a song file from this PC -----
  const pickSong = async () => {
    try {
      const [path] = await api.app.pickFiles('Pick a song (MP3, M4A, WAV, FLAC or OGG)');
      if (!path) return;
      const t = await api.karaoke.track(path);
      wantPlay.current = false;
      setPlaying(false);
      setTrack(t);
      setFileLyrics({ status: 'loading' });
      setSource('file');
    } catch (e) {
      toast.error('Couldn’t open that song', errorText(e));
    }
  };

  const loadFileLyrics = useCallback(
    async (durationMs: number) => {
      if (!track) return;
      setFileLyrics({ status: 'loading' });
      try {
        const l = await api.karaoke.lyrics({ song: track.path, title: track.title, artist: track.artist, durationMs });
        setFileLyrics(l && l.lines.length ? { status: 'ready', lyrics: l } : { status: 'none', lyrics: l ?? undefined });
      } catch (e) {
        setFileLyrics({ status: 'error', error: errorText(e) });
      }
    },
    [track],
  );

  const pickLrc = async () => {
    try {
      const [path] = await api.app.pickFiles('Pick a lyrics file (.lrc)');
      if (!path) return;
      const l = await api.karaoke.lyrics({ lrc: path, title, artist, durationMs: 0 });
      if (l) setOwnLyrics({ key: songKey, lyrics: l });
    } catch (e) {
      toast.error('Couldn’t use that lyrics file', errorText(e));
    }
  };

  // ----- the microphone -----
  const [mic, setMic] = useState<MicInput | null>(null);
  const [micState, setMicState] = useState<'off' | 'starting' | 'on' | MicError>('off');
  const [mics, setMics] = useState<MicDevice[]>([]);
  const frame = useRef<VoiceFrame | null>(null);
  const coach = useRef<Coach | null>(null);
  const [summary, setSummary] = useState<{ s: CoachSummary; retry: { i: number; text: string }[]; mode: Mode } | null>(null);
  const follower = useRef(new Follower([]));
  const [followState, setFollowState] = useState<FollowState>('free');

  const canFollowVoice = source === 'file' && !!track && lines.length > 0;
  const following = mode === 'voice' && canFollowVoice && micState === 'on';

  // Fresh values for the microphone's frame handler.
  const live = useRef({ lines, time, following, speed, mode, title, artist });
  live.current = { lines, time, following, speed, mode, title, artist };

  useEffect(() => {
    follower.current.setLines(followLines(lines));
  }, [lines]);
  useEffect(() => {
    if (coach.current) coach.current.measureTiming = mode === 'song';
  }, [mode]);
  useEffect(() => {
    if (source === 'now' && mode === 'voice') setMode('song');
  }, [source, mode, setMode]);

  const saveSession = useCallback(async (c: Coach, t: string, a: string, m: Mode, ls: LyricLine[], showIt: boolean) => {
    const s = c.summary();
    const retry = s.retry.map((i) => ({ i, text: ls[i]?.text ?? '' })).filter((r) => r.text);
    if (showIt) setSummary({ s, retry, mode: m });
    if (c.singingSeconds < 8) return;
    try {
      setData(await api.karaoke.addSession({ at: Date.now(), title: t, artist: a, mode: m, seconds: s.seconds, lines: s.lines, steadinessCents: s.steadinessCents, timingMs: s.timingMs, lowNote: s.lowNote, highNote: s.highNote, retry: retry.map((r) => r.text) }));
    } catch (e) {
      toast.error('Couldn’t save the practice', errorText(e));
    }
  }, []);

  const startMic = async (id = micId) => {
    setMicState('starting');
    try {
      const m = await MicInput.start(id || undefined);
      m.onEnded = () => {
        setMic(null);
        setMicState('missing');
        if (coach.current) void saveSession(coach.current, live.current.title, live.current.artist, live.current.mode, live.current.lines, true);
        coach.current = null;
      };
      coach.current = new Coach(mode === 'song');
      setSummary(null);
      setMic(m);
      setMicState('on');
      void listMics().then(setMics);
    } catch (e) {
      setMicState(typeof e === 'string' ? (e as MicError) : 'unsupported');
    }
  };

  const stopMic = useCallback(
    (showSummary = true) => {
      mic?.stop();
      setMic(null);
      setMicState('off');
      frame.current = null;
      const c = coach.current;
      coach.current = null;
      if (c) void saveSession(c, live.current.title, live.current.artist, live.current.mode, live.current.lines, showSummary);
      const a = audio.current;
      if (a) {
        a.playbackRate = speed;
        a.volume = 1;
      }
    },
    [mic, saveSession, speed],
  );

  // Leaving the studio stops the microphone.
  const micRef = useRef(mic);
  micRef.current = mic;
  useEffect(() => () => micRef.current?.stop(), []);

  // A new song while practising: save the last one's practice and go on.
  const lastSong = useRef({ key: songKey, title, artist, lines });
  useEffect(() => {
    const prev = lastSong.current;
    if (prev.key !== songKey && coach.current) {
      void saveSession(coach.current, prev.title, prev.artist, live.current.mode, prev.lines, false);
      coach.current = new Coach(live.current.mode === 'song');
      follower.current.reset();
    }
    lastSong.current = { key: songKey, title, artist, lines };
  }, [songKey, title, artist, lines, saveSession]);

  // Every microphone frame: measure, and (Follow my voice) steer the music.
  useEffect(() => {
    if (!mic) return;
    return mic.listen((f) => {
      frame.current = f;
      const L = live.current;
      const t = L.time();
      const p = phaseAt(L.lines, t);
      const idx = p.kind === 'line' && t <= lineEnd(L.lines, p.index) + 400 ? p.index : -1;
      coach.current?.add(f, idx, idx >= 0 ? L.lines[idx].ms : 0, t);
      const a = audio.current;
      if (!L.following || !a) return;
      const o = follower.current.step({ now: f.at, song: a.currentTime * 1000, voiced: f.voiced, onset: f.onset, speed: L.speed, playing: wantPlay.current });
      if (o.seek !== null) a.currentTime = Math.max(0, o.seek / 1000);
      a.playbackRate = o.rate;
      a.volume = o.volume;
      if (o.pause && !a.paused) a.pause();
      else if (!o.pause && a.paused && wantPlay.current) void a.play().catch(() => undefined);
      setFollowState((s) => (s === o.state ? s : o.state));
    });
  }, [mic]);

  // Not following: the song plays at the practice speed, full volume.
  useEffect(() => {
    const a = audio.current;
    if (!a || following) return;
    a.playbackRate = speed;
    a.volume = 1;
    setFollowState('free');
    follower.current.reset();
    if (wantPlay.current && a.paused) void a.play().catch(() => undefined);
  }, [following, speed]);

  // ----- transport -----
  const seekTo = useCallback(
    (ms: number) => {
      const at = Math.max(0, ms);
      if (source === 'file') {
        if (audio.current) audio.current.currentTime = at / 1000;
        follower.current.reset();
      } else np.control('seek', at);
    },
    [source, np],
  );

  const togglePlay = () => {
    if (source === 'now') return np.control('toggle');
    const a = audio.current;
    if (!a) return;
    wantPlay.current = !wantPlay.current;
    setPlaying(wantPlay.current);
    if (wantPlay.current) {
      if (following && follower.current.current === 'held') return; // waits for your voice
      void a.play().catch(() => undefined);
    } else a.pause();
  };

  const releaseHold = () => {
    const at = follower.current.release();
    const a = audio.current;
    if (!a) return;
    if (at !== null) a.currentTime = at / 1000;
    wantPlay.current = true;
    setPlaying(true);
    void a.play().catch(() => undefined);
  };

  // Loop a line: back to just before it once it's sung.
  useEffect(() => {
    if (loop === null || !lines[loop]) return;
    const id = window.setInterval(() => {
      const t = time();
      if (t > lineEnd(lines, loop) + 350 || t < lines[loop].ms - 2500) seekTo(lines[loop].ms - 900);
    }, 120);
    return () => window.clearInterval(id);
  }, [loop, lines, time, seekTo]);

  const currentLine = () => {
    const p = phaseAt(lines, time());
    return p.kind === 'line' ? p.index : p.kind === 'break' ? p.next : -1;
  };
  const practice = (i: number) => {
    setLoop(i);
    seekTo(lines[i].ms - 900);
    if (source === 'file' && !wantPlay.current) togglePlay();
  };

  // ----- what to show -----
  const npStatus = np.lyrics?.status;
  const noLyricsText =
    source === 'file'
      ? fileLyrics.status === 'loading'
        ? 'Looking for lyrics…'
        : fileLyrics.status === 'error'
          ? `Couldn’t look for lyrics: ${fileLyrics.error}`
          : fileLyrics.lyrics?.plain
            ? 'Only untimed lyrics were found for this song, so they can’t follow it. Pick an .lrc file with timings.'
            : 'No timed lyrics found for this song. Put an .lrc file with the same name beside it, or pick one.'
      : !np.state
        ? 'Nothing is playing. Play a song in Spotify, Apple Music, YouTube or any player, or pick a song file from this PC.'
        : npStatus === 'searching'
          ? 'Looking for lyrics…'
          : npStatus === 'off'
            ? 'Lyrics are turned off in the music player’s settings.'
            : npStatus === 'ready' && np.lyrics?.status === 'ready' && np.lyrics.lyrics.instrumental
              ? 'This is an instrumental.'
              : npStatus === 'ready'
                ? 'Only untimed lyrics were found for this song, so they can’t follow it. Pick an .lrc file with timings.'
                : 'No timed lyrics found for this song. Pick an .lrc file if you have one.';

  const accentNote =
    reader === 'neutral'
      ? `Pronunciation can’t be written in ${readerName} spelling yet, so a standard romanization (Latin letters) is used.`
      : lang && lang.code.includes('-')
        ? `Written for ${readerName} in general; differences between accents (${lang.name.match(/\((.*)\)/)?.[1] ?? lang.name} and others) aren’t modelled yet.`
        : '';

  if (!data) return <div className="h-[420px]" />;

  return (
    <div className="space-y-4">
      <LanguageSetup
        open={setup}
        first={!data.profile}
        current={data.profile?.lang ?? null}
        onClose={() => setSetup(false)}
        onSave={async (code) => {
          try {
            setData(await api.karaoke.setLanguage(code));
            setSetup(false);
          } catch (e) {
            toast.error('Couldn’t save your language', errorText(e));
          }
        }}
      />

      {!data.profile ? (
        <Card className="p-6">
          <EmptyState icon={Languages} title="Choose your first language" description="Karaoke Studio writes how to say each word the way you read." action={<Button variant="primary" onClick={() => setSetup(true)}>Choose language</Button>} />
        </Card>
      ) : (
        <>
          {/* Song, mode and microphone */}
          <Card className="p-4">
            <div className="flex flex-wrap items-center gap-2">
              <Segmented
                label="Song"
                value={source}
                onChange={(v) => (v === 'file' && !track ? void pickSong() : setSource(v))}
                options={[
                  { value: 'now', label: 'Now playing', icon: ListMusic },
                  { value: 'file', label: 'Song from this PC', icon: FileAudio },
                ]}
              />
              {source === 'file' && (
                <Button size="sm" variant="ghost" icon={FileAudio} onClick={() => void pickSong()}>
                  {track ? 'Another song' : 'Pick a song'}
                </Button>
              )}
              <div className="mx-1 h-6 w-px bg-line" aria-hidden />
              <Segmented
                label="Mode"
                value={mode}
                onChange={setMode}
                options={[
                  { value: 'song', label: 'Follow the song', icon: Play },
                  { value: 'voice', label: 'Follow my voice', icon: Waves, disabled: source !== 'file', title: source !== 'file' ? 'Needs a song file from this PC: Spotify, Apple Music and other apps can’t be slowed or held smoothly from here' : undefined },
                ]}
              />
              <div className="ml-auto flex items-center gap-2">
                {micState === 'on' ? (
                  <>
                    <span className="inline-flex items-center gap-1.5 rounded-full bg-bad/12 px-2.5 py-1 text-[12px] font-medium text-bad" title="Analysed on this PC, never recorded or sent">
                      <span className="h-2 w-2 animate-pulse rounded-full bg-bad" aria-hidden /> Listening
                    </span>
                    <Button size="sm" icon={Square} onClick={() => stopMic()}>
                      Finish
                    </Button>
                  </>
                ) : (
                  <Button size="sm" variant="primary" icon={Mic} loading={micState === 'starting'} onClick={() => void startMic()}>
                    Start singing
                  </Button>
                )}
                <IconButton icon={Languages} label={`Your language: ${lang?.name ?? '—'}`} onClick={() => setSetup(true)} />
              </div>
            </div>
            {typeof micState === 'string' && !['off', 'starting', 'on'].includes(micState) && (
              <Callout tone="warn" icon={MicOff} className="mt-3" action={<Button size="sm" onClick={() => void startMic()}>Try again</Button>}>
                {micErrorText(micState as MicError)} You can still sing along without it; nothing will be measured.
              </Callout>
            )}
            {mode === 'voice' && source === 'file' && (
              <p className="mt-3 flex items-start gap-2 text-[12.5px] text-dim">
                <Headphones size={14} className="mt-0.5 shrink-0" aria-hidden />
                {micState === 'on' ? 'The song waits for you before each line and when you stop, and goes on from where you stopped. ' : 'Start singing to turn the microphone on; the song then follows you. '}
                Use headphones: with speakers the microphone also hears the song and can’t tell it from you.
              </p>
            )}
          </Card>

          <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_320px]">
            {/* Lyrics */}
            <Card className="min-h-[440px] p-4">
              <div className="flex flex-wrap items-center gap-2">
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[15px] font-semibold text-fg">{title || (source === 'file' ? 'No song picked' : 'Nothing playing')}</div>
                  <div className="truncate text-[12.5px] text-dim">
                    {artist}
                    {source === 'now' && np.state?.appName ? `${artist ? ' · ' : ''}${np.state.appName}` : ''}
                  </div>
                </div>
                {lines.length > 0 && (
                  <>
                    <Select value={songLang} onChange={(e) => setSongLang(e.target.value)} aria-label="Song language" className="h-8 w-auto py-0 text-[12.5px]">
                      <option value="auto">{`Language: ${detected.name} (detected)`}</option>
                      {SONG_LANGS.map((l) => (
                        <option key={l.code} value={l.code}>
                          {l.name}
                        </option>
                      ))}
                    </Select>
                    {lyricSource && <Badge tone="neutral">Lyrics: {lyricSource}</Badge>}
                    {estimated && (
                      <Badge tone="warn" title="These lyrics only time lines; each word’s time is estimated from its syllables. Use “Lyrics early/late” to correct.">
                        Approximate word timing
                      </Badge>
                    )}
                  </>
                )}
                <IconButton icon={FileText} label="Use my own lyrics file (.lrc)" onClick={() => void pickLrc()} disabled={!songKey} />
              </div>

              {lines.length > 0 ? (
                <>
                  <KaraokeLyrics lines={lines} time={time} song={song} reader={reader} show={show} loop={loop} waiting={following && (followState === 'held' || followState === 'waiting')} onPick={(i) => seekTo(lines[i].ms - 900)} />
                  <p className="text-center text-[11.5px] text-faint">{guideNote(song, reader, readerName)}</p>
                </>
              ) : (
                <EmptyState compact icon={ListMusic} title={source === 'file' && !track ? 'Pick a song from this PC' : 'No lyrics to follow yet'} description={source === 'file' && !track ? 'Songs OmniHub plays itself can be slowed, looped and can follow your voice.' : noLyricsText} action={source === 'file' && !track ? <Button onClick={() => void pickSong()}>Pick a song</Button> : undefined} className="py-16" />
              )}

              {/* Transport */}
              {songKey && (
                <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-line pt-3">
                  <IconButton icon={(source === 'file' ? playing : np.state?.playing) ? Pause : Play} label={(source === 'file' ? playing : np.state?.playing) ? 'Pause' : 'Play'} onClick={togglePlay} variant="secondary" />
                  {source === 'file' && <FileProgress audio={audio} duration={duration} onSeek={seekTo} />}
                  {following && followState === 'held' && (
                    <Button size="sm" icon={SkipForward} onClick={releaseHold}>
                      Go on without me
                    </Button>
                  )}
                  <Button size="sm" variant={loop !== null ? 'primary' : 'ghost'} icon={Repeat} disabled={!lines.length} onClick={() => setLoop(loop !== null ? null : currentLine() >= 0 ? currentLine() : null)}>
                    {loop !== null ? 'Looping line' : 'Loop line'}
                  </Button>
                  <div className="ml-auto flex items-center gap-1.5 text-[12px] text-dim">
                    <span>Lyrics</span>
                    <Button size="xs" variant="ghost" onClick={() => setOffset(offset - 100)} title="The lyrics come too late: show them earlier">
                      earlier
                    </Button>
                    <span className="w-12 text-center tabular-nums">{offset === 0 ? '±0' : `${offset > 0 ? '+' : ''}${(offset / 1000).toFixed(1)} s`}</span>
                    <Button size="xs" variant="ghost" onClick={() => setOffset(offset + 100)} title="The lyrics come too early: show them later">
                      later
                    </Button>
                    {offset !== 0 && <IconButton size="sm" icon={RotateCcw} label="Reset lyrics timing" onClick={() => setOffset(0)} />}
                  </div>
                </div>
              )}
              {source === 'file' && track && (
                <audio
                  ref={audio}
                  src={track.url}
                  preload="auto"
                  onLoadedMetadata={(e) => {
                    const a = e.currentTarget;
                    a.preservesPitch = true;
                    setDuration(a.duration * 1000);
                    void loadFileLyrics(a.duration * 1000);
                  }}
                  onEnded={() => {
                    wantPlay.current = false;
                    setPlaying(false);
                  }}
                  onError={() => setFileLyrics({ status: 'error', error: 'This song file can’t be played here' })}
                  className="hidden"
                />
              )}
            </Card>

            {/* Voice, practice and options */}
            <div className="space-y-4">
              <Card className="p-4">
                <CardHeader icon={Mic} title="Your voice" subtitle={micState === 'on' ? (mic?.label ? mic.label : 'Microphone on') : 'Analysed on this PC, never recorded.'} />
                <div className="mt-3">
                  {micState === 'on' ? <VoiceMeter frame={frame} solfege={solfege} /> : <p className="text-[12.5px] text-faint">Press Start singing to see your pitch, how close you are to each note, and to get a summary at the end.</p>}
                </div>
                {following && (
                  <div className="mt-3 rounded-lg bg-surface-2/60 px-3 py-2 text-[12.5px] text-dim">
                    {followState === 'held' ? 'Waiting for you…' : followState === 'waiting' ? 'Your line is next…' : followState === 'slowing' ? 'You stopped: easing off' : followState === 'singing' || followState === 'breath' ? 'Following you' : 'Playing (no lyrics here)'}
                  </div>
                )}
                {mics.length > 1 && (
                  <Select
                    value={micId}
                    onChange={(e) => {
                      setMicId(e.target.value);
                      if (mic) {
                        stopMic(false);
                        void startMic(e.target.value);
                      }
                    }}
                    aria-label="Microphone"
                    className="mt-3 h-8 py-0 text-[12.5px]"
                  >
                    <option value="">Windows’ default microphone</option>
                    {mics.map((m) => (
                      <option key={m.id} value={m.id}>
                        {m.label}
                      </option>
                    ))}
                  </Select>
                )}
              </Card>

              <Card className="p-4">
                <CardHeader icon={Languages} title="Pronunciation" subtitle={`For ${lang?.name ?? 'your language'}`} actions={<Button size="xs" variant="ghost" onClick={() => setSetup(true)}>Change</Button>} />
                <div className="mt-3 space-y-2.5">
                  <div className="flex items-center justify-between gap-3 text-[13px] text-fg">
                    How to say each word
                    <Switch checked={show.say} onChange={(v) => setShow({ ...show, say: v })} label="How to say each word" />
                  </div>
                  <div className="flex items-center justify-between gap-3 text-[13px] text-fg">
                    <span>
                      Syllables <span className="text-faint">(stressed one in capitals when known)</span>
                    </span>
                    <Switch checked={show.syllables} onChange={(v) => setShow({ ...show, syllables: v })} label="Syllables" />
                  </div>
                </div>
                {accentNote && <p className="mt-3 text-[11.5px] text-faint">{accentNote}</p>}
              </Card>

              {source === 'file' && track && (
                <Card className="p-4">
                  <CardHeader icon={Gauge} title="Practice speed" subtitle="Slower without changing the key." />
                  <Segmented
                    label="Practice speed"
                    className="mt-3"
                    size="sm"
                    value={String(speed)}
                    onChange={(v) => setSpeed(Number(v))}
                    options={[
                      { value: '0.5', label: '50%' },
                      { value: '0.75', label: '75%' },
                      { value: '0.9', label: '90%' },
                      { value: '1', label: '100%' },
                    ]}
                  />
                </Card>
              )}
              {source === 'now' && np.state && <p className="px-1 text-[11.5px] text-faint">Slowing down and Follow my voice need a song file from this PC: other players can’t be slowed or held smoothly from here.</p>}
            </div>
          </div>

          {summary && <SummaryCard s={summary.s} retry={summary.retry} mode={summary.mode} solfege={solfege} onClose={() => setSummary(null)} onPractice={practice} />}

          <HistoryCard
            sessions={data.sessions}
            solfege={solfege}
            onClear={() =>
              void confirm({ title: 'Delete your practice history?', description: 'Every practice summary is deleted from this PC. Your language stays.', confirmLabel: 'Delete', tone: 'danger' }).then(async (ok) => {
                if (!ok) return;
                try {
                  setData(await api.karaoke.clear());
                  toast.success('Practice history deleted');
                } catch (e) {
                  toast.error('Couldn’t delete it', errorText(e));
                }
              })
            }
          />
        </>
      )}
    </div>
  );
}

/** Where a song file is, and a bar to move through it. */
function FileProgress({ audio, duration, onSeek }: { audio: React.RefObject<HTMLAudioElement | null>; duration: number; onSeek: (ms: number) => void }) {
  const [t, setT] = useState(0);
  useEffect(() => {
    const id = window.setInterval(() => setT((audio.current?.currentTime ?? 0) * 1000), 250);
    return () => window.clearInterval(id);
  }, [audio]);
  return (
    <div className="flex min-w-[200px] flex-1 items-center gap-2 text-[12px] tabular-nums text-dim">
      <span>{fmtTime(t)}</span>
      <input type="range" min={0} max={Math.max(1, duration)} step={250} value={Math.min(t, duration)} onChange={(e) => onSeek(Number(e.target.value))} aria-label="Position in the song" className="h-1 flex-1 accent-[var(--accent)]" />
      <span>{fmtTime(duration)}</span>
    </div>
  );
}
