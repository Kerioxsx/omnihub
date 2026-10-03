import { AnimatePresence, motion } from 'motion/react';
import { useEffect } from 'react';
import { Page } from '../../components/Page';
import { Skeleton } from '../../components/ui/Card';
import { useInterval } from '../../lib/hooks';
import { useLive } from '../../state/live';
import { CreateVault } from './CreateVault';
import { LockScreen } from './LockScreen';
import { VaultUnlocked } from './VaultUnlocked';

export function VaultPage() {
  const status = useLive((s) => s.vault);
  const refresh = useLive((s) => s.refreshVault);
  useEffect(() => {
    void refresh();
  }, [refresh]);
  // Keep locksIn / retryAfter fresh.
  useInterval(() => void refresh(), 10000);

  const state = !status ? 'loading' : !status.exists ? 'create' : status.unlocked ? 'open' : 'locked';
  const subtitle = { loading: 'Checking the vault…', create: 'Encrypted passwords, cards and notes — stored only on this PC.', locked: 'Locked. Enter your master password to continue.', open: 'Unlocked. It locks itself when you step away.' }[state];

  return (
    <Page title="Vault" subtitle={subtitle} scroll={state === 'create'}>
      <AnimatePresence mode="wait">
        <motion.div key={state} className="flex min-h-0 flex-1 flex-col" initial={{ opacity: 0, scale: 0.99 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: 0.99 }} transition={{ duration: 0.18 }}>
          {state === 'loading' && <Skeleton className="h-[420px] rounded-2xl" />}
          {state === 'create' && <CreateVault />}
          {state === 'locked' && status && <LockScreen status={status} />}
          {state === 'open' && status && <VaultUnlocked status={status} />}
        </motion.div>
      </AnimatePresence>
    </Page>
  );
}
