import type { VaultStatus } from '@shared/types';
import { Download, KeyRound, Upload } from 'lucide-react';
import { type ReactNode, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Field, Select, Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { useLive } from '../../state/live';
import { useSettings } from '../../state/settings';
import { toast } from '../../state/toasts';
import { StrengthMeter, useStrength } from './shared';

function Row({ title, hint, children }: { title: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 py-3">
      <div>
        <div className="text-[13.5px] font-medium text-fg">{title}</div>
        {hint && <div className="text-[12px] text-faint">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

export function VaultSettings({ open, onClose, status }: { open: boolean; onClose: () => void; status: VaultStatus }) {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [oldPw, setOldPw] = useState('');
  const [newPw, setNewPw] = useState('');
  const [importPw, setImportPw] = useState('');
  const [importPath, setImportPath] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const strength = useStrength(newPw);
  if (!settings) return null;

  const run = async (key: string, fn: () => Promise<void>) => {
    setBusy(key);
    try {
      await fn();
    } catch (e) {
      toast.error('Something went wrong', errorText(e));
    } finally {
      setBusy(null);
    }
  };

  return (
    <Modal open={open} onClose={onClose} title="Vault settings" icon={KeyRound} size="lg">
      <div className="divide-y divide-line pb-2">
        <Row title="Lock after inactivity" hint="Counted from your last action in OmniHub">
          <Select value={String(settings.vault.autoLockMinutes)} onChange={(e) => void update({ vault: { autoLockMinutes: Number(e.target.value) } })} className="w-[140px]" aria-label="Auto-lock">
            {[1, 2, 5, 10, 15, 30, 60].map((m) => (
              <option key={m} value={m}>
                {m} minute{m > 1 ? 's' : ''}
              </option>
            ))}
          </Select>
        </Row>
        <Row title="Clear copied secrets after" hint="The clipboard is wiped if it still holds the secret">
          <Select value={String(settings.vault.clipboardClearSeconds)} onChange={(e) => void update({ vault: { clipboardClearSeconds: Number(e.target.value) } })} className="w-[140px]" aria-label="Clipboard clear">
            {[10, 15, 20, 30, 45, 60, 90].map((s) => (
              <option key={s} value={s}>
                {s} seconds
              </option>
            ))}
          </Select>
        </Row>
        <Row title="Lock when Windows locks" hint="Win+L, sleep or switching users">
          <Switch checked={settings.vault.lockOnSessionLock} onChange={(v) => void update({ vault: { lockOnSessionLock: v } })} label="Lock when Windows locks" />
        </Row>
        <Row title="Windows Hello" hint={status.helloAvailable ? 'Unlock with your face, fingerprint or PIN' : 'Not available on this PC'}>
          <Switch
            checked={status.helloEnabled}
            disabled={!status.helloAvailable || busy === 'hello'}
            onChange={(v) =>
              void run('hello', async () => {
                if (v) await api.vault.helloEnable();
                else await api.vault.helloDisable();
                await useLive.getState().refreshVault();
                toast.success(v ? 'Windows Hello enabled' : 'Windows Hello disabled');
              })
            }
            label="Windows Hello"
          />
        </Row>
        <div className="py-4">
          <div className="mb-3 flex items-center gap-2 text-[13.5px] font-medium text-fg">
            <KeyRound size={15} className="text-accent" /> Change master password
          </div>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Current" htmlFor="vs-old">
              <TextInput id="vs-old" type="password" value={oldPw} onChange={(e) => setOldPw(e.target.value)} autoComplete="current-password" />
            </Field>
            <Field label="New" htmlFor="vs-new">
              <TextInput id="vs-new" type="password" value={newPw} onChange={(e) => setNewPw(e.target.value)} autoComplete="new-password" />
            </Field>
          </div>
          {newPw && (
            <div className="mt-2">
              <StrengthMeter strength={strength} compact />
            </div>
          )}
          <Button
            className="mt-3"
            size="sm"
            loading={busy === 'pw'}
            disabled={!oldPw || (strength?.score ?? 0) < 2}
            onClick={() =>
              void run('pw', async () => {
                await api.vault.changePassword(oldPw, newPw);
                setOldPw('');
                setNewPw('');
                toast.success('Master password changed');
              })
            }
          >
            Change password
          </Button>
        </div>
        <div className="py-4">
          <div className="mb-1 text-[13.5px] font-medium text-fg">Encrypted backup</div>
          <div className="mb-3 text-[12px] text-faint">The backup stays encrypted with your master password — keep it somewhere safe, like a USB stick.</div>
          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              icon={Download}
              loading={busy === 'export'}
              onClick={() =>
                void run('export', async () => {
                  const path = await api.app.saveFile('Export vault backup', `omnihub-vault-${new Date().toISOString().slice(0, 10)}.ohvault`);
                  if (!path) return;
                  await api.vault.exportBackup(path);
                  toast.success('Backup exported', path);
                })
              }
            >
              Export backup
            </Button>
            <Button
              size="sm"
              icon={Upload}
              onClick={() =>
                void run('pick', async () => {
                  const [p] = await api.app.pickFiles('Choose a vault backup');
                  if (p) setImportPath(p);
                })
              }
            >
              Import backup…
            </Button>
          </div>
          {importPath && (
            <div className="mt-3 flex items-end gap-2 rounded-xl border border-line bg-surface p-3">
              <Field label={`Password for ${importPath.split('\\').pop()}`} htmlFor="vs-import" className="flex-1">
                <TextInput id="vs-import" type="password" value={importPw} onChange={(e) => setImportPw(e.target.value)} />
              </Field>
              <Button
                variant="primary"
                size="md"
                icon={Upload}
                loading={busy === 'import'}
                disabled={!importPw}
                onClick={() =>
                  void run('import', async () => {
                    const n = await api.vault.importBackup(importPath, importPw);
                    toast.success(`Imported ${n} entries`);
                    setImportPath(null);
                    setImportPw('');
                  })
                }
              >
                Import
              </Button>
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
