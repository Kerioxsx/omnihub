// One game's options: start fullscreen, a RetroArch shader, more options
// for the emulator, cheats (written to RetroArch's cheat folder), and
// RetroArch netplay (host, or join a friend's address).

import type { EmuState, Netplay, Rom, RomOpts } from '@shared/types';
import { Plus, Settings2, Trash2, Users } from 'lucide-react';
import { useState } from 'react';
import { api, errorText } from '../../../api';
import { Button, IconButton } from '../../../components/ui/Button';
import { Field, Switch, TextInput } from '../../../components/ui/Form';
import { Modal } from '../../../components/ui/Overlay';
import { toast } from '../../../state/toasts';

const EMPTY: RomOpts = { fullscreen: false, shader: '', extra: '', cheats: [] };

export function RomOptions({ rom, state, onClose, onState }: { rom: Rom; state: EmuState; onClose: () => void; onState: (s: EmuState) => void }) {
  const [o, setO] = useState<RomOpts>(state.romOpts[rom.path] ?? EMPTY);
  const [address, setAddress] = useState('');
  const emu = state.emulators.find((e) => e.id === state.chosen[rom.platform]) ?? state.emulators.find((e) => e.platforms.includes(rom.platform));
  const retroarch = emu?.name === 'RetroArch';
  const save = async () => {
    try {
      onState(await api.emulators.setRomOpts(rom.id, o));
      toast.success('Saved', retroarch && o.cheats.length ? 'Cheats: in RetroArch, Quick Menu → Cheats → Load Cheat File → OmniHub.' : undefined);
      onClose();
    } catch (e) {
      toast.error('Couldn’t save', errorText(e));
    }
  };
  const play = (n: Netplay) =>
    void api.emulators.launch(rom.id, n).then(
      () => toast.success(n.kind === 'host' ? 'Hosting: friends join your address' : 'Joining…', n.kind === 'host' ? 'On the internet, forward port 55435 on your router.' : undefined),
      (e: unknown) => toast.error('Couldn’t start it', errorText(e)),
    );
  return (
    <Modal
      open
      onClose={onClose}
      icon={Settings2}
      title={rom.name.replace(/\s*\(.*?\)/g, '')}
      description={emu ? `Plays in ${emu.name}.` : 'No emulator for this platform yet.'}
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={() => void save()}>
            Save
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <label className="flex items-center justify-between gap-3 text-[13.5px] text-fg">
          Start in fullscreen
          <Switch checked={o.fullscreen} onChange={(v) => setO({ ...o, fullscreen: v })} label="Start in fullscreen" />
        </label>
        {retroarch && (
          <Field label="Shader (RetroArch)" hint="A CRT or smoothing look: pick a .slangp or .glslp from RetroArch’s shaders folder.">
            <div className="flex gap-2">
              <TextInput value={o.shader} readOnly placeholder="None" className="flex-1" />
              <Button
                onClick={async () => {
                  const [f] = await api.app.pickFiles('Pick a shader preset');
                  if (f) setO({ ...o, shader: f });
                }}
              >
                Pick…
              </Button>
              {o.shader && <IconButton icon={Trash2} label="No shader" onClick={() => setO({ ...o, shader: '' })} />}
            </div>
          </Field>
        )}
        <Field label="More options for the emulator" hint="Passed as they are, before the game, like --log-level 1.">
          <TextInput value={o.extra} maxLength={300} onChange={(e) => setO({ ...o, extra: e.target.value })} />
        </Field>
        <div>
          <div className="mb-1.5 flex items-center justify-between">
            <span className="text-[13px] font-semibold text-fg">Cheats</span>
            <Button size="sm" variant="ghost" icon={Plus} onClick={() => setO({ ...o, cheats: [...o.cheats, { name: '', code: '', on: true }] })}>
              Add
            </Button>
          </div>
          <p className="mb-2 text-[12px] text-faint">{retroarch ? 'Saved as a RetroArch cheat file (Quick Menu → Cheats → Load Cheat File → OmniHub).' : 'Kept here to copy into the emulator’s own cheat menu.'}</p>
          <div className="space-y-2">
            {o.cheats.map((c, i) => (
              <div key={i} className="flex items-center gap-2">
                <Switch checked={c.on} onChange={(v) => setO({ ...o, cheats: o.cheats.map((x, k) => (k === i ? { ...x, on: v } : x)) })} label="On" size="md" />
                <TextInput value={c.name} placeholder="Name" maxLength={80} onChange={(e) => setO({ ...o, cheats: o.cheats.map((x, k) => (k === i ? { ...x, name: e.target.value } : x)) })} className="w-40" inputSize="sm" />
                <TextInput value={c.code} placeholder="Code" onChange={(e) => setO({ ...o, cheats: o.cheats.map((x, k) => (k === i ? { ...x, code: e.target.value } : x)) })} className="flex-1 font-mono" inputSize="sm" />
                <IconButton icon={Trash2} label="Remove" onClick={() => setO({ ...o, cheats: o.cheats.filter((_, k) => k !== i) })} />
              </div>
            ))}
          </div>
        </div>
        {retroarch && (
          <div className="rounded-xl border border-line p-3">
            <div className="mb-2 flex items-center gap-2 text-[13px] font-semibold text-fg">
              <Users size={14} /> Netplay (RetroArch)
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button size="sm" onClick={() => play({ kind: 'host' })}>
                Host
              </Button>
              <TextInput value={address} onChange={(e) => setAddress(e.target.value)} placeholder="Friend’s address, like 203.0.113.5" className="min-w-[200px] flex-1" inputSize="sm" />
              <Button size="sm" disabled={!address.trim()} onClick={() => play({ kind: 'join', address: address.trim() })}>
                Join
              </Button>
            </div>
            <p className="mt-2 text-[12px] text-faint">Both of you need the same game file and RetroArch core. Over the internet the host forwards port 55435 on their router.</p>
          </div>
        )}
      </div>
    </Modal>
  );
}
