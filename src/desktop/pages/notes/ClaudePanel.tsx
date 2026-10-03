import { formatBytes, formatRelative } from '@shared/format';
import type { FolderFile } from '@shared/types';
import { AnimatePresence, motion } from 'motion/react';
import { Bot, FileText, FolderOpen, RefreshCw, X } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Skeleton, Spinner } from '../../components/ui/Card';
import { Modal } from '../../components/ui/Overlay';
import { EmptyState, ErrorState } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { useAsync, useEvent } from '../../lib/hooks';
import { renderMarkdown } from '../../lib/util';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';

export function ClaudePanel({ onClose }: { onClose: () => void }) {
  const folder = useSettings((s) => s.settings?.notes.claudeFolder ?? null);
  const update = useSettings((s) => s.update);
  const files = useAsync(() => (folder ? api.notes.folderFiles() : Promise.resolve([] as FolderFile[])), [folder]);
  const [fresh, setFresh] = useState<Set<string>>(new Set());
  const [preview, setPreview] = useState<FolderFile | null>(null);
  const content = useAsync(() => (preview ? api.notes.folderRead(preview.path) : Promise.resolve('')), [preview?.path]);

  useEvent('notes:folder-changed', async () => {
    const before = new Set((files.data ?? []).map((f) => f.path));
    const next = await api.notes.folderFiles().catch(() => null);
    if (!next) return;
    files.setData(next);
    const added = next.filter((f) => !before.has(f.path));
    if (added.length) {
      setFresh((s) => new Set([...s, ...added.map((f) => f.path)]));
      const reply = added.find((f) => !f.fromOmnihub);
      if (reply) toast.info('New file from Claude', reply.name, { action: { label: 'Open', run: () => setPreview(reply) } });
    }
  });

  const choose = async () => {
    const f = await api.app.pickFolder('Choose the Claude ideas folder');
    if (f) await update({ notes: { claudeFolder: f } });
  };

  const list = files.data ?? [];
  const replies = list.filter((f) => !f.fromOmnihub).length;

  return (
    <div className="card flex min-h-0 flex-col overflow-hidden">
      <div className="flex items-start gap-2 border-b border-line px-4 py-3">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-[10px] bg-accent-soft text-accent">
          <Bot size={16} aria-hidden />
        </div>
        <div className="min-w-0 flex-1">
          <div className="text-[13.5px] font-semibold text-fg">Claude folder</div>
          <div className="truncate font-mono text-[11px] text-faint" title={folder ?? ''}>
            {folder ?? 'Not set'}
          </div>
        </div>
        {folder && <IconButton icon={FolderOpen} label="Open folder" size="sm" onClick={() => void api.app.openPath(folder).catch((e: unknown) => toast.error('Could not open', errorText(e)))} />}
        <IconButton icon={RefreshCw} label="Refresh" size="sm" onClick={() => void files.reload()} />
        <IconButton icon={X} label="Hide panel" size="sm" onClick={onClose} />
      </div>
      {!folder ? (
        <EmptyState compact icon={FolderOpen} title="No folder yet" description="Pick the folder Claude reads. Replies Claude writes there show up here." action={<Button size="sm" variant="primary" onClick={() => void choose()}>Choose folder</Button>} />
      ) : files.error ? (
        <ErrorState error={files.error} onRetry={files.reload} />
      ) : !files.data ? (
        <div className="space-y-2 p-3">
          {[0, 1, 2, 3].map((i) => (
            <Skeleton key={i} className="h-14" />
          ))}
        </div>
      ) : (
        <>
          {replies > 0 && (
            <div className="border-b border-line px-4 py-2 text-[12px] text-dim">
              <span className="font-medium text-accent">{replies}</span> file{replies === 1 ? '' : 's'} written by Claude
            </div>
          )}
          <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
            <AnimatePresence initial={false}>
              {list.map((f) => (
                <motion.button
                  key={f.path}
                  layout="position"
                  type="button"
                  initial={fresh.has(f.path) ? { opacity: 0, x: 12 } : false}
                  animate={{ opacity: 1, x: 0 }}
                  onClick={() => setPreview(f)}
                  className={cx('mb-0.5 block w-full rounded-xl px-3 py-2 text-left transition-colors hover:bg-surface-2', !f.fromOmnihub && 'bg-accent-soft/60 ring-1 ring-accent/25', fresh.has(f.path) && 'ring-1 ring-accent')}
                >
                  <div className="flex items-center gap-2">
                    {f.fromOmnihub ? <FileText size={14} className="shrink-0 text-faint" aria-hidden /> : <Bot size={14} className="shrink-0 text-accent" aria-hidden />}
                    <span className="truncate text-[12.5px] font-medium text-fg">{f.name}</span>
                  </div>
                  <div className="mt-0.5 line-clamp-2 pl-[22px] text-[11.5px] leading-snug text-faint">{f.preview}</div>
                  <div className="mt-1 flex items-center gap-2 pl-[22px] text-[10.5px] text-faint">
                    <span>{formatRelative(f.modified)}</span>
                    <span>{formatBytes(f.size)}</span>
                    {!f.fromOmnihub && (
                      <Badge tone="accent" className="h-[17px] px-1.5 text-[10px]">
                        from Claude
                      </Badge>
                    )}
                  </div>
                </motion.button>
              ))}
            </AnimatePresence>
            {!list.length && <div className="px-3 py-6 text-center text-[12.5px] text-faint">The folder is empty. Send an idea to start.</div>}
          </div>
        </>
      )}
      <Modal open={!!preview} onClose={() => setPreview(null)} title={preview?.name} description={preview ? `${preview.fromOmnihub ? 'Written by OmniHub' : 'Not written by OmniHub — probably a reply from Claude'} · ${formatRelative(preview.modified)}` : undefined} icon={preview?.fromOmnihub ? FileText : Bot} size="lg">
        {content.loading && !content.data ? <Spinner /> : content.error ? <ErrorState error={content.error} /> : preview?.name.endsWith('.md') ? <div className="markdown pb-3" dangerouslySetInnerHTML={{ __html: renderMarkdown(content.data ?? '') }} /> : <pre className="whitespace-pre-wrap pb-3 font-mono text-[12px] text-dim">{content.data}</pre>}
      </Modal>
    </div>
  );
}
