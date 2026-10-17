import { basename } from '@shared/format';
import { ClipboardPaste, FilePlus, FolderPlus, Power, Send, Type, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../api';
import { navigate } from '../lib/router';
import { useLive } from '../state/live';
import { useSend } from '../state/send';
import { useSettings } from '../state/settings';
import { toast } from '../state/toasts';
import { FileIcon } from './FileIcon';
import { Button, IconButton } from './ui/Button';
import { Segmented, Select, Textarea } from './ui/Form';
import { Modal } from './ui/Overlay';
import { Callout } from './ui/States';

const looksLikeFolder = (p: string) => !/\.[a-z0-9]{1,6}$/i.test(basename(p));

/** Send files, folders (zipped) or text to a paired phone's inbox. */
export function SendDialog() {
  const { open, mode, paths, text, setPaths, setText, setMode, close } = useSend();
  const devices = useLive((s) => s.devices).filter((d) => !d.revoked);
  const running = useLive((s) => !!s.remote?.running);
  const update = useSettings((s) => s.update);
  const [target, setTarget] = useState('');
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (open && target && !devices.some((d) => d.id === target)) setTarget('');
  }, [open, devices, target]);

  const add = async (kind: 'files' | 'folder') => {
    try {
      const more = kind === 'files' ? await api.app.pickFiles('Choose files to send') : await api.app.pickFolder('Choose a folder to send (it is zipped)').then((p) => (p ? [p] : []));
      setPaths([...new Set([...paths, ...more])]);
    } catch (e) {
      toast.error('Could not open the picker', errorText(e));
    }
  };

  const paste = async () => {
    const t = await api.app.clipboardText().catch(() => null);
    if (t) setText(t);
    else toast.info('The clipboard has no text');
  };

  const targetName = target ? (devices.find((d) => d.id === target)?.name ?? 'the phone') : devices.length === 1 ? devices[0].name : 'every paired phone';

  const send = async () => {
    setBusy(true);
    try {
      if (mode === 'files') {
        const items = await api.remote.send(paths, target || null);
        toast.success(`Sent ${items.length} item${items.length === 1 ? '' : 's'} to ${targetName}`, 'On the phone: Home → From your PC.');
      } else {
        await api.remote.sendText(text, target || null);
        toast.success(`Sent to ${targetName}`, 'On the phone: Home → From your PC, with a Copy button.');
      }
      setPaths([]);
      setText('');
      close();
    } catch (e) {
      toast.error('Could not send', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const ready = running && devices.length > 0 && (mode === 'files' ? paths.length > 0 : text.trim().length > 0);

  return (
    <Modal
      open={open}
      onClose={close}
      title="Send to your phone"
      description="It waits in the phone's inbox (Home → From your PC) until you open it there."
      icon={Send}
      footer={
        <>
          <Button variant="ghost" onClick={close}>
            Cancel
          </Button>
          <Button variant="primary" icon={Send} disabled={!ready} loading={busy} onClick={() => void send()}>
            {mode === 'files' ? `Send ${paths.length || ''}` : 'Send text'}
          </Button>
        </>
      }
    >
      <div className="space-y-4 pb-2">
        {!running ? (
          <Callout tone="warn" icon={Power} title="The phone companion is off" action={<Button size="sm" variant="primary" onClick={() => void update({ remote: { enabled: true } })}>Turn it on</Button>}>
            Phones can only collect what you send while it runs.
          </Callout>
        ) : devices.length === 0 ? (
          <Callout
            tone="info"
            title="No phone is paired yet"
            action={
              <Button
                size="sm"
                variant="primary"
                onClick={() => {
                  close();
                  navigate('phone', { pair: '1' });
                }}
              >
                Pair a phone
              </Button>
            }
          >
            Pair your phone once, then send as often as you like.
          </Callout>
        ) : null}

        <Segmented
          label="What to send"
          value={mode}
          onChange={setMode}
          options={[
            { value: 'files', label: 'Files & folders', icon: FilePlus },
            { value: 'text', label: 'Text or link', icon: Type },
          ]}
        />

        {mode === 'files' ? (
          <div className="space-y-1.5">
            {paths.map((p) => (
              <div key={p} className="flex items-center gap-2.5 rounded-lg border border-line bg-surface py-1 pl-3 pr-1">
                <FileIcon name={basename(p)} isDir={looksLikeFolder(p)} />
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[13px] text-fg">{basename(p)}</div>
                  <div className="truncate font-mono text-[11px] text-faint">
                    {p}
                    {looksLikeFolder(p) ? ' · sent as a .zip' : ''}
                  </div>
                </div>
                <IconButton icon={X} label="Remove" size="sm" onClick={() => setPaths(paths.filter((x) => x !== p))} />
              </div>
            ))}
            {!paths.length && <div className="rounded-xl border border-dashed border-line px-4 py-6 text-center text-[13px] text-faint">Drop files on the OmniHub window, or choose them below.</div>}
            <div className="flex gap-2 pt-1">
              <Button size="xs" icon={FilePlus} onClick={() => void add('files')}>
                Add files…
              </Button>
              <Button size="xs" icon={FolderPlus} onClick={() => void add('folder')}>
                Add a folder…
              </Button>
            </div>
          </div>
        ) : (
          <div className="space-y-2">
            <Textarea value={text} onChange={(e) => setText(e.target.value)} rows={5} placeholder="A link, an address, a code, a note…" aria-label="Text to send" className="font-[inherit]" />
            <Button size="xs" icon={ClipboardPaste} onClick={() => void paste()}>
              Use what's on the clipboard
            </Button>
          </div>
        )}

        {devices.length > 1 && (
          <label className="block text-[12.5px] text-dim">
            Send to
            <Select value={target} onChange={(e) => setTarget(e.target.value)} className="mt-1.5" aria-label="Phone">
              <option value="">Every paired phone</option>
              {devices.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.name}
                </option>
              ))}
            </Select>
          </label>
        )}
      </div>
    </Modal>
  );
}
