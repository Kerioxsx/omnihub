// File kind → icon and colour.

import { Folder, Image, Film, Music, FileText, FileArchive, AppWindow, File as FileGeneric, FileCode, FileSpreadsheet } from 'lucide-react';
import type { FsEntry } from '../client';
import { cx } from '../lib/util';

export type Kind = FsEntry['kind'] | 'code' | 'sheet';

const EXT: Record<string, Kind> = {};
const add = (k: Kind, exts: string) => exts.split(' ').forEach((e) => (EXT[e] = k));
add('image', 'jpg jpeg png gif webp bmp heic heif svg avif tif tiff');
add('video', 'mp4 mkv mov avi webm wmv m4v');
add('audio', 'mp3 flac wav ogg m4a aac opus wma');
add('document', 'pdf doc docx txt md odt rtf ppt pptx epub');
add('sheet', 'xls xlsx csv ods');
add('archive', 'zip 7z rar tar gz xz bz2 iso');
add('app', 'exe msi apk msix appx');
add('code', 'js ts tsx jsx rs py c cpp h cs java go json xml html css sh ps1 toml yaml yml');

export function kindOf(name: string, kind?: FsEntry['kind']): Kind {
  const ext = name.includes('.') ? name.split('.').pop()!.toLowerCase() : '';
  const byExt = EXT[ext];
  if (kind === 'folder') return 'folder';
  if (byExt && (kind === undefined || kind === 'file' || kind === 'document')) return byExt;
  return kind ?? byExt ?? 'file';
}

const STYLE: Record<Kind, { icon: typeof Folder; cls: string }> = {
  folder: { icon: Folder, cls: 'bg-violet-500/15 text-violet-400' },
  image: { icon: Image, cls: 'bg-emerald-500/15 text-emerald-400' },
  video: { icon: Film, cls: 'bg-pink-500/15 text-pink-400' },
  audio: { icon: Music, cls: 'bg-amber-500/15 text-amber-400' },
  document: { icon: FileText, cls: 'bg-sky-500/15 text-sky-400' },
  sheet: { icon: FileSpreadsheet, cls: 'bg-green-500/15 text-green-400' },
  archive: { icon: FileArchive, cls: 'bg-orange-500/15 text-orange-400' },
  app: { icon: AppWindow, cls: 'bg-slate-400/15 text-slate-400' },
  code: { icon: FileCode, cls: 'bg-blue-500/15 text-blue-400' },
  file: { icon: FileGeneric, cls: 'bg-surface-3 text-dim' },
};

export function FileIcon({ name, kind, size = 44, className }: { name: string; kind?: FsEntry['kind']; size?: number; className?: string }) {
  const k = kindOf(name, kind);
  const s = STYLE[k];
  const Icon = s.icon;
  return (
    <div className={cx('grid shrink-0 place-items-center rounded-xl', s.cls, className)} style={{ width: size, height: size }}>
      <Icon size={Math.round(size * 0.46)} strokeWidth={1.9} />
    </div>
  );
}
