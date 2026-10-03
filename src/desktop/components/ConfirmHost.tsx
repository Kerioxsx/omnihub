import { CircleHelp, TriangleAlert } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useConfirm } from '../state/dialogs';
import { Button } from './ui/Button';
import { TextInput } from './ui/Form';
import { Modal } from './ui/Overlay';

export function ConfirmHost() {
  const req = useConfirm((s) => s.request);
  const close = useConfirm((s) => s.close);
  const [typed, setTyped] = useState('');
  useEffect(() => setTyped(''), [req]);
  const danger = req?.tone === 'danger';
  const blocked = !!req?.typed && typed.trim() !== req.typed;
  return (
    <Modal
      open={!!req}
      onClose={() => close(false)}
      title={req?.title}
      description={req?.description}
      icon={danger ? TriangleAlert : CircleHelp}
      iconTone={danger ? 'bad' : 'accent'}
      size="md"
      footer={
        <>
          <Button variant="ghost" onClick={() => close(false)}>
            {req?.cancelLabel ?? 'Cancel'}
          </Button>
          <Button variant={danger ? 'danger' : 'primary'} disabled={blocked} onClick={() => close(true)} data-autofocus={req?.typed ? undefined : true}>
            {req?.confirmLabel ?? 'Confirm'}
          </Button>
        </>
      }
    >
      {req?.details && <div className="max-h-64 overflow-y-auto rounded-xl border border-line bg-surface p-3 text-[13px]">{req.details}</div>}
      {req?.typed && (
        <form
          className="mt-3"
          onSubmit={(e) => {
            e.preventDefault();
            if (!blocked) close(true);
          }}
        >
          <label htmlFor="confirm-typed" className="mb-1.5 block text-[12.5px] text-dim">
            Type <span className="rounded bg-surface-3 px-1.5 py-0.5 font-mono text-fg">{req.typed}</span> to confirm
          </label>
          <TextInput id="confirm-typed" value={typed} onChange={(e) => setTyped(e.target.value)} autoComplete="off" spellCheck={false} data-autofocus />
        </form>
      )}
      {!req?.details && !req?.typed && <div className="h-1" />}
    </Modal>
  );
}
