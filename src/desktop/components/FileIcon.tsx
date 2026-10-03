import type { LucideIcon } from 'lucide-react';
import { Database, File, FileArchive, FileCode, FileCog, FileImage, FileMusic, FilePlay, FileText, Folder, FolderOpen } from 'lucide-react';
import { extColor } from '@shared/format';

const BY_COLOR: Record<string, LucideIcon> = {
  'var(--tm-video)': FilePlay,
  'var(--tm-image)': FileImage,
  'var(--tm-audio)': FileMusic,
  'var(--tm-archive)': FileArchive,
  'var(--tm-code)': FileCode,
  'var(--tm-doc)': FileText,
  'var(--tm-exe)': FileCog,
  'var(--tm-data)': Database,
};

export function extOf(name: string): string | null {
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) return null;
  const ext = name.slice(dot + 1);
  return ext.length > 16 || ext.includes(' ') ? null : ext.toLowerCase();
}

/** Folder or file-type icon tinted with the treemap palette. */
export function FileIcon({ name, isDir, size = 16, open }: { name: string; isDir: boolean; size?: number; open?: boolean }) {
  if (isDir) {
    const I = open ? FolderOpen : Folder;
    return <I size={size} aria-hidden className="shrink-0 text-[#e8b04b] light:text-[#c98a12]" fill="currentColor" fillOpacity={0.18} />;
  }
  const color = extColor(extOf(name));
  const I = BY_COLOR[color] ?? File;
  return <I size={size} aria-hidden className="shrink-0" style={{ color }} />;
}
