// Family and guest phones: limit a paired phone to some areas (a sibling's
// phone that can only use the deck and the music, say). Anything else is
// refused on the PC, including areas added in later versions.

import type { Device } from '@shared/types';
import { Shield } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Checkbox, Switch } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { toast } from '../../state/toasts';

export function PhoneLimitsDialog({ device, open, onClose }: { device: Device; open: boolean; onClose: () => void }) {
  const [areas, setAreas] = useState<[string, string][]>([]);
  const [limited, setLimited] = useState(false);
  const [picked, setPicked] = useState<string[]>([]);
  useEffect(() => {
    if (!open) return;
    void api.phoneLimits.get().then((r) => {
      setAreas(r.areas);
      const mine = r.phones[device.id];
      setLimited(!!mine);
      setPicked(mine ?? ['deck', 'music']);
    }, () => undefined);
  }, [open, device.id]);
  const save = async () => {
    try {
      await api.phoneLimits.set(device.id, limited ? picked : null);
      toast.success(limited ? `${device.name} is limited` : `${device.name} can use everything again`);
      onClose();
    } catch (e) {
      toast.error('Couldn’t save', errorText(e));
    }
  };
  return (
    <Modal
      open={open}
      onClose={onClose}
      icon={Shield}
      title={`What ${device.name} can do`}
      description="For family or guest phones: limit this phone to a few areas. Everything else is refused on the PC."
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
      <div className="flex items-center justify-between gap-3 text-[13.5px] text-fg">
        Limit this phone
        <Switch checked={limited} onChange={setLimited} label="Limit this phone" />
      </div>
      {limited && (
        <div className="mt-3 space-y-2 rounded-xl border border-line p-3">
          {areas.map(([id, label]) => (
            <Checkbox key={id} checked={picked.includes(id)} onChange={(v) => setPicked(v ? [...picked, id] : picked.filter((x) => x !== id))} label={label} />
          ))}
          <p className="pt-1 text-[11.5px] text-faint">The vault is never available on a limited phone.</p>
        </div>
      )}
    </Modal>
  );
}
