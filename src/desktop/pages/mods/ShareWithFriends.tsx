// Share a mix or an installed modpack with friends: one file with the mods
// (Modrinth's as downloads, the rest inside), the configs, the cover, and
// optionally your server in their server list.

import type { McPackFormat, McServerEntry, McShared, SrvView } from '@shared/types';
import { Gift, Globe, House } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Segmented, Select, Switch, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { toast } from '../../state/toasts';
import { count } from './shared';

const mb = (b: number) => `${(b / 1_048_576).toFixed(b < 10_485_760 ? 1 : 0)} MB`;

export function ShareWithFriends({ open, onClose, name, formats, save }: { open: boolean; onClose: () => void; name: string; formats: boolean; save: (path: string, format: McPackFormat, server: McServerEntry | null) => Promise<McShared | void> }) {
  const [format, setFormat] = useState<McPackFormat>('mrpack');
  const [withServer, setWithServer] = useState(false);
  const [servers, setServers] = useState<SrvView[] | null>(null);
  const [serverId, setServerId] = useState('');
  const [address, setAddress] = useState('');
  const [choices, setChoices] = useState<{ label: string; value: string; icon: typeof House }[]>([]);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!open || !withServer || servers) return;
    void api.servers.list().then((l) => {
      setServers(l);
      setServerId((cur) => cur || l[0]?.id || '');
    }, () => setServers([]));
  }, [open, withServer, servers]);

  useEffect(() => {
    const s = servers?.find((x) => x.id === serverId);
    if (!s) return;
    const home = s.lan ? [{ label: `Same Wi-Fi: ${s.lan}`, value: s.lan, icon: House }] : [];
    setChoices(home);
    setAddress(s.lan ?? '');
    void api.servers.network(s.id).then((n) => {
      if (n.publicIp && !n.cgnat) setChoices([...home, { label: `Over the internet: ${n.publicIp}:${n.port}`, value: `${n.publicIp}:${n.port}`, icon: Globe }]);
    }, () => undefined);
  }, [servers, serverId]);

  const go = async () => {
    const ext = format === 'curseforge' ? 'zip' : 'mrpack';
    const path = await api.app.saveFile('Save the pack for your friends', `${name.replace(/[^\w\s+-]/g, '').trim() || 'Pack'}.${ext}`);
    if (!path) return;
    const server = withServer && address.trim() ? { name: servers?.find((x) => x.id === serverId)?.name ?? name, address: address.trim() } : null;
    setBusy(true);
    try {
      const r = await save(path, format, server);
      const what = r ? `${count(r.mods)} mods (${r.fromModrinth} downloaded from Modrinth when they install it, ${r.included} inside), ${mb(r.bytes)}.` : 'Mods, configs and the cover are in it.';
      toast.success('Ready to send', `${what} Send them the file: it opens in OmniHub (Install a friend’s pack), the Modrinth App or Prism Launcher${format === 'curseforge' ? ', or CurseForge (Import)' : ''}.`);
      onClose();
    } catch (e) {
      toast.error('Not saved', errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={`Share ${name} with friends`}
      description="One file with everything: the mods, their configs, resource and shader packs, and the cover. Your own settings, keys and server list stay out."
      icon={Gift}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" icon={Gift} loading={busy} disabled={busy || (withServer && !address.trim())} onClick={() => void go()}>
            Save the file
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        {formats && (
          <div className="space-y-1.5">
            <div className="text-[12.5px] font-medium text-fg">For their launcher</div>
            <Segmented
              label="File kind"
              value={format}
              onChange={setFormat}
              options={[
                { value: 'mrpack', label: '.mrpack (OmniHub, Modrinth App, Prism)' },
                { value: 'curseforge', label: 'CurseForge zip' },
              ]}
            />
            {format === 'curseforge' && <div className="text-[11.5px] text-faint">Every mod goes inside the zip, so it's bigger.</div>}
          </div>
        )}
        <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
          <span>
            Put my server in their server list
            <span className="block text-[11.5px] text-faint">They find it in Multiplayer, ready to join.</span>
          </span>
          <Switch checked={withServer} onChange={setWithServer} label="Add my server" />
        </label>
        {withServer &&
          (servers && servers.length === 0 ? (
            <div className="space-y-1.5">
              <div className="text-[12px] text-dim">No server on this PC yet. Type its address:</div>
              <TextInput value={address} onChange={(e) => setAddress(e.target.value)} placeholder="play.example.net or 203.0.113.7:25565" aria-label="Server address" />
            </div>
          ) : (
            <div className="space-y-2">
              <Select value={serverId} onChange={(e) => setServerId(e.target.value)} aria-label="Server">
                {(servers ?? []).map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.name}
                  </option>
                ))}
              </Select>
              <div className="flex flex-wrap gap-1.5">
                {choices.map((c) => (
                  <Button key={c.value} size="sm" variant={address === c.value ? 'secondary' : 'ghost'} icon={c.icon} onClick={() => setAddress(c.value)}>
                    {c.label}
                  </Button>
                ))}
              </div>
              <TextInput value={address} onChange={(e) => setAddress(e.target.value)} placeholder="Address friends join with" aria-label="Server address" />
            </div>
          ))}
      </div>
    </Modal>
  );
}
