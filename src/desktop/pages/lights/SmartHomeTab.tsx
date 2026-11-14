// Smart home: Home Assistant (your server's address and a long-lived token
// from your profile there; kept encrypted on this PC) and Shelly plugs on
// your network. Switch them here, or from scenes and deck buttons
// ("Smart home device").

import type { HomeDevice, HomeStatus } from '@shared/types';
import { Home, Plug, Plus, Power, RefreshCw, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button, IconButton } from '../../components/ui/Button';
import { Card, CardHeader } from '../../components/ui/Card';
import { TextInput } from '../../components/ui/Form';
import { toast } from '../../state/toasts';

export function SmartHomeTab() {
  const [st, setSt] = useState<HomeStatus | null>(null);
  const [devices, setDevices] = useState<HomeDevice[]>([]);
  const [url, setUrl] = useState('http://homeassistant.local:8123');
  const [token, setToken] = useState('');
  const [plug, setPlug] = useState({ name: '', ip: '' });
  const [busy, setBusy] = useState(false);
  const load = useCallback(async () => {
    try {
      setDevices(await api.home.devices());
    } catch (e) {
      toast.error('Couldn’t list the devices', errorText(e));
    }
  }, []);
  useEffect(() => {
    void api.home.status().then((s) => {
      setSt(s);
      if (s.connected || s.shelly.length) void load();
    }, () => undefined);
  }, [load]);
  const connect = async (t: string) => {
    setBusy(true);
    try {
      const s = await api.home.connect(url, t);
      setSt(s);
      setToken('');
      if (s.connected) void load();
      else setDevices((d) => d.filter((x) => !x.id.startsWith('ha:')));
    } catch (e) {
      toast.error('Couldn’t connect', errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const savePlugs = async (plugs: [string, string][]) => {
    try {
      setSt(await api.home.setShelly(plugs));
      void load();
    } catch (e) {
      toast.error('Couldn’t save the plug', errorText(e));
    }
  };
  const run = async (d: HomeDevice, action: 'on' | 'off' | 'toggle') => {
    try {
      await api.home.run(d.id, action);
      if (d.id.startsWith('ha:') && d.state) setDevices((l) => l.map((x) => (x.id === d.id ? { ...x, state: action === 'toggle' ? (x.state === 'on' ? 'off' : 'on') : action } : x)));
    } catch (e) {
      toast.error(`Couldn’t switch ${d.name}`, errorText(e));
    }
  };
  if (!st) return null;
  return (
    <div className="space-y-4">
      <Card className="p-5">
        <CardHeader
          icon={Home}
          title="Home Assistant"
          subtitle={st.connected ? `Connected to ${st.url}.` : 'Your Home Assistant’s address and a long-lived access token (your profile in Home Assistant → Security → Long-lived access tokens). The token stays on this PC, encrypted.'}
          actions={
            st.connected ? (
              <Button size="sm" variant="ghost" onClick={() => void connect('')}>
                Disconnect
              </Button>
            ) : undefined
          }
        />
        {!st.connected && (
          <div className="mt-3 flex flex-wrap gap-2">
            <TextInput value={url} onChange={(e) => setUrl(e.target.value)} placeholder="http://homeassistant.local:8123" className="min-w-[220px] flex-1" />
            <TextInput value={token} onChange={(e) => setToken(e.target.value)} placeholder="Long-lived access token" type="password" className="min-w-[220px] flex-1" />
            <Button variant="primary" loading={busy} disabled={!token.trim()} onClick={() => void connect(token)}>
              Connect
            </Button>
          </div>
        )}
      </Card>
      <Card className="p-5">
        <CardHeader icon={Plug} title="Shelly plugs" subtitle="Shelly smart plugs on your network, switched directly (no account). Give each one a fixed address in your router." />
        <div className="mt-3 flex flex-wrap gap-2">
          <TextInput value={plug.name} onChange={(e) => setPlug({ ...plug, name: e.target.value })} placeholder="Name, like Desk lamp" className="w-44" />
          <TextInput value={plug.ip} onChange={(e) => setPlug({ ...plug, ip: e.target.value })} placeholder="192.168.1.40" className="w-40" />
          <Button
            icon={Plus}
            disabled={!plug.name.trim() || !plug.ip.trim()}
            onClick={() => {
              void savePlugs([...st.shelly, [plug.name.trim(), plug.ip.trim()]]);
              setPlug({ name: '', ip: '' });
            }}
          >
            Add
          </Button>
        </div>
      </Card>
      {devices.length > 0 && (
        <Card className="p-5">
          <CardHeader
            icon={Power}
            title="Devices"
            subtitle="Lights, switches, fans, scenes and scripts. Door locks, alarms and garage doors aren’t offered here on purpose."
            actions={<IconButton icon={RefreshCw} label="Refresh" onClick={() => void load()} />}
          />
          <div className="mt-3 divide-y divide-line rounded-xl border border-line">
            {devices.map((d) => (
              <div key={d.id} className="flex items-center gap-3 px-3 py-2 text-[13px]">
                <span className="w-24 shrink-0 text-[11.5px] uppercase tracking-wider text-faint">{d.kind.replace('_', ' ')}</span>
                <span className="min-w-0 flex-1 truncate text-fg">{d.name}</span>
                {d.state && <span className={d.state === 'on' ? 'text-good' : 'text-faint'}>{d.state === 'on' ? 'On' : 'Off'}</span>}
                {d.kind === 'scene' || d.kind === 'script' ? (
                  <Button size="sm" onClick={() => void run(d, 'on')}>
                    Run
                  </Button>
                ) : (
                  <Button size="sm" onClick={() => void run(d, 'toggle')}>
                    Toggle
                  </Button>
                )}
                {d.id.startsWith('shelly:') && <IconButton icon={Trash2} label={`Remove ${d.name}`} size="sm" onClick={() => void savePlugs(st.shelly.filter(([, ip]) => `shelly:${ip}` !== d.id))} />}
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}
