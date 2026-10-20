// Setup: finding the lights on the network (LAN Control, Windows Firewall,
// lights added by address) and the Govee API key for scenes and the rest.

import type { FirewallReport } from '@shared/types';
import { Cloud, ExternalLink, KeyRound, Plus, RefreshCw, ShieldCheck, Trash2, Wifi } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Badge, Card, CardHeader } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { Callout } from '../../components/ui/States';
import { toast } from '../../state/toasts';
import type { Govee } from './useGovee';

export function SetupTab({ govee }: { govee: Govee }) {
  const s = govee.settings;
  const st = govee.status;
  const [fw, setFw] = useState<FirewallReport | null>(null);
  const [key, setKey] = useState('');
  const [addr, setAddr] = useState('');
  useEffect(() => {
    void api.govee.firewall().then(setFw, () => undefined);
  }, []);
  if (!s) return null;

  const saveKey = async (k: string) => {
    try {
      await api.govee.setKey(k);
      setKey('');
      if (k) toast.info('Key saved', 'Encrypted for your Windows account.');
      else toast.info('Key removed');
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader
          icon={Wifi}
          title="On your network"
          subtitle="Lights with LAN Control answer OmniHub directly: no account, no cloud, and fast enough for live colours."
          className="px-5 pt-4"
          actions={
            <Button size="sm" variant="secondary" icon={RefreshCw} onClick={() => void api.govee.scan().then(() => toast.info('Looking for lights…'))}>
              Look again
            </Button>
          }
        />
        <div className="space-y-3 px-5 pb-4 pt-3">
          <div className="flex flex-wrap items-center gap-2 text-[12.5px] text-dim">
            <Badge tone={st?.lan === 'on' ? 'good' : st?.lan === 'error' ? 'bad' : 'neutral'}>{st?.lan === 'on' ? 'Listening' : st?.lan === 'error' ? 'Not listening' : 'Off'}</Badge>
            {st?.lights.filter((l) => l.lan).length ?? 0} found on the network
          </div>
          {st?.lanMessage && <Callout tone="bad">{st.lanMessage}</Callout>}
          <Callout tone="info" title="A light isn’t found?">
            In the Govee Home app, open the light → settings (⚙) → switch on <b>LAN Control</b>. The PC and the light need to be on the same Wi-Fi network.
          </Callout>
          {fw && fw.supported && fw.verdict !== 'allowed' && fw.verdict !== 'off' && (
            <Callout
              tone="warn"
              title="Windows Firewall may block the lights’ answers"
              action={
                <Button size="sm" variant="secondary" icon={ShieldCheck} onClick={() => void api.govee.fixFirewall().then(setFw, (e: unknown) => toast.error('Not allowed', errorText(e)))}>
                  Allow
                </Button>
              }
            >
              {fw.message} Allowing it adds a rule for OmniHub on private networks (it asks for administrator approval once).
            </Callout>
          )}
          <div className="space-y-2">
            <div className="text-[12.5px] font-medium text-fg">Lights by address</div>
            <div className="text-[11.5px] text-faint">For a light on another part of the network: its IP address (Govee Home app → the light → settings → Device info).</div>
            {s.manual.map((a) => (
              <div key={a} className="flex items-center justify-between rounded-lg bg-surface-2/60 px-3 py-1.5 font-mono text-[12.5px]">
                {a}
                <IconButton icon={Trash2} label="Remove" size="sm" onClick={() => void govee.update({ manual: s.manual.filter((x) => x !== a) })} />
              </div>
            ))}
            <div className="flex gap-2">
              <TextInput value={addr} onChange={(e) => setAddr(e.target.value)} placeholder="192.168.1.50" className="w-48 font-mono" aria-label="Address" />
              <Button
                size="sm"
                variant="secondary"
                icon={Plus}
                disabled={!/^\d{1,3}(\.\d{1,3}){3}$/.test(addr.trim())}
                onClick={() => {
                  void govee.update({ manual: [...s.manual, addr.trim()] }).then(() => api.govee.scan());
                  setAddr('');
                }}
              >
                Add
              </Button>
            </div>
          </div>
        </div>
      </Card>

      <Card>
        <CardHeader icon={Cloud} title="Govee account (optional)" subtitle="With your Govee API key: each light’s own scenes and DIY scenes, its own music modes, and settings such as gradient, night light or a speaker’s volume where the light offers them." className="px-5 pt-4" />
        <div className="space-y-3 px-5 pb-4 pt-3">
          <div className="flex flex-wrap items-center gap-2 text-[12.5px] text-dim">
            <Badge tone={st?.cloud === 'on' ? 'good' : st?.cloud === 'error' ? 'bad' : 'neutral'}>{st?.hasKey ? (st.cloud === 'on' ? 'Connected' : st.cloud === 'error' ? 'Problem' : 'Key saved') : 'No key'}</Badge>
            {st?.cloud === 'on' && `${st.lights.filter((l) => l.cloud).length} lights in your account`}
          </div>
          {st?.cloudMessage && <Callout tone="bad">{st.cloudMessage}</Callout>}
          <div className="flex flex-wrap gap-2">
            <TextInput type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder={st?.hasKey ? '•••••••• (saved)' : 'Paste your Govee API key'} icon={KeyRound} className="min-w-[260px] flex-1" aria-label="Govee API key" autoComplete="off" />
            <Button size="sm" variant="primary" disabled={!key.trim()} onClick={() => void saveKey(key)}>
              Save
            </Button>
            {st?.hasKey && (
              <Button size="sm" variant="ghost" onClick={() => void saveKey('')}>
                Remove
              </Button>
            )}
          </div>
          <p className="text-[11.5px] leading-relaxed text-faint">
            Get one in the Govee Home app: Profile → Settings (⚙) → <b>Apply for API Key</b>; Govee emails it. It’s kept encrypted for your Windows account, shown to no one, and sent only to Govee. Govee allows about two changes a second per light this way, so live colours always go over your network.{' '}
            <button type="button" className="inline-flex items-center gap-0.5 text-accent hover:underline" onClick={() => void api.app.openUrl('https://developer.govee.com/')}>
              developer.govee.com <ExternalLink size={11} />
            </button>
          </p>
        </div>
      </Card>
    </div>
  );
}
