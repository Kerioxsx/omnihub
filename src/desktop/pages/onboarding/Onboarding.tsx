import { AnimatePresence, motion } from 'motion/react';
import type { LucideIcon } from 'lucide-react';
import { ArrowLeft, ArrowRight, Bot, Camera, Check, FolderOpen, HardDrive, LayoutGrid, LockKeyhole, NotebookPen, ShieldCheck, Smartphone, Wifi, Zap } from 'lucide-react';
import { type ReactNode, useState } from 'react';
import { api } from '../../api';
import { Logo } from '../../components/Logo';
import { Button } from '../../components/ui/Button';
import { Switch } from '../../components/ui/Form';
import { cx } from '../../lib/cx';
import { useSettings } from '../../state/settings';

const FEATURES: [LucideIcon, string, string][] = [
  [HardDrive, 'Storage', 'Treemap, cleanup, duplicates'],
  [LayoutGrid, 'Apps', 'Everything installed, real sizes'],
  [Camera, 'Screenshots', 'Capture, tag, find again'],
  [NotebookPen, 'Notes', 'Markdown + ideas for Claude'],
  [LockKeyhole, 'Vault', 'Encrypted, local only'],
  [Smartphone, 'Phone', 'Files, power, screen'],
];

function Step({ icon: Icon, title, children }: { icon?: LucideIcon; title: ReactNode; children: ReactNode }) {
  return (
    <div>
      {Icon && (
        <div className="accent-gradient mb-5 flex h-14 w-14 items-center justify-center rounded-2xl text-white shadow-[0_14px_40px_-12px_var(--accent-glow)]">
          <Icon size={26} aria-hidden />
        </div>
      )}
      <h2 className="font-display text-[26px] font-semibold leading-tight tracking-tight text-fg">{title}</h2>
      <div className="mt-3 text-[14px] leading-relaxed text-dim">{children}</div>
    </div>
  );
}

export function Onboarding({ onDone }: { onDone: () => void }) {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const [step, setStep] = useState(0);
  const [dir, setDir] = useState(1);
  const total = 4;

  const go = (n: number) => {
    setDir(n > step ? 1 : -1);
    setStep(n);
  };
  const finish = async () => {
    await update({ general: { onboarded: true } }, { silent: true });
    onDone();
  };
  const pick = async () => {
    const f = await api.app.pickFolder('Choose the Claude ideas folder');
    if (f) await update({ notes: { claudeFolder: f } }, { silent: true });
  };

  const steps: ReactNode[] = [
    <div key="welcome">
      <motion.div initial={{ scale: 0.8, rotate: -8, opacity: 0 }} animate={{ scale: 1, rotate: 0, opacity: 1 }} transition={{ type: 'spring', stiffness: 200, damping: 16 }} className="mb-6 inline-block">
        <Logo size={72} />
      </motion.div>
      <Step title={<span className="gradient-text">Welcome to OmniHub</span>}>Your PC, organised: see what fills your drives, keep screenshots and ideas findable, store secrets safely and use your phone as a remote. Everything stays on this PC.</Step>
      <div className="mt-6 grid grid-cols-3 gap-2.5">
        {FEATURES.map(([I, t, d], i) => (
          <motion.div key={t} initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0, transition: { delay: 0.15 + i * 0.05 } }} className="rounded-xl border border-line bg-surface px-3 py-2.5">
            <I size={16} className="text-accent" aria-hidden />
            <div className="mt-1.5 text-[13px] font-semibold text-fg">{t}</div>
            <div className="text-[11.5px] text-faint">{d}</div>
          </motion.div>
        ))}
      </div>
    </div>,
    <div key="admin">
      <Step icon={ShieldCheck} title="No admin rights — except when you ask">
        OmniHub runs as a normal app. Only the <span className="font-medium text-fg">Fast scan</span> needs administrator rights, because Windows lets only administrators read the NTFS Master File Table directly.
      </Step>
      <div className="mt-5 space-y-2.5">
        {[
          [Zap, 'Fast scan asks every time', 'You approve a Windows (UAC) prompt; only the scanner runs elevated, and it exits when done.'],
          [HardDrive, 'Standard scan never asks', 'It walks folders with your normal permissions — slower, but no prompt.'],
          [ShieldCheck, 'Nothing elevated in the background', 'Some cleanup items (Windows Update files) need a manual “Restart as administrator”.'],
        ].map(([I, t, d]) => {
          const Icon = I as LucideIcon;
          return (
            <div key={t as string} className="flex gap-3 rounded-xl border border-line bg-surface px-4 py-3">
              <Icon size={17} className="mt-0.5 shrink-0 text-accent" aria-hidden />
              <div>
                <div className="text-[13.5px] font-semibold text-fg">{t as string}</div>
                <div className="text-[12.5px] text-dim">{d as string}</div>
              </div>
            </div>
          );
        })}
      </div>
    </div>,
    <div key="claude">
      <Step icon={Bot} title="Hand ideas to Claude">
        Write ideas as Markdown notes, then <span className="font-medium text-fg">Send to Claude</span> drops each one as a file into a folder Claude can read — for example a project folder Claude Code works in. Replies Claude writes there show up next to your ideas.
      </Step>
      <div className="mt-6 rounded-2xl border border-line bg-surface p-4">
        <div className="text-[12px] font-medium uppercase tracking-wider text-faint">Claude folder (optional)</div>
        <div className="mt-2 flex items-center gap-3">
          <div className={cx('min-w-0 flex-1 truncate rounded-xl border border-line bg-surface-2 px-3 py-2 font-mono text-[12.5px]', settings?.notes.claudeFolder ? 'text-fg' : 'text-faint')}>{settings?.notes.claudeFolder ?? 'Not chosen — you can do this later'}</div>
          <Button icon={FolderOpen} onClick={() => void pick()}>
            {settings?.notes.claudeFolder ? 'Change' : 'Choose…'}
          </Button>
        </div>
      </div>
    </div>,
    <div key="phone">
      <Step icon={Smartphone} title="Your phone as a remote">
        The phone companion is a small web server on your Wi-Fi. Paired phones can browse and send files, put the PC to sleep (with a countdown you can cancel), watch the screen and jot notes — from any browser, no app needed.
      </Step>
      <div className="mt-6 flex items-center gap-4 rounded-2xl border border-line bg-surface p-4">
        <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-surface-3 text-dim">
          <Wifi size={18} aria-hidden />
        </div>
        <div className="flex-1">
          <div className="text-[13.5px] font-semibold text-fg">Off until you turn it on</div>
          <div className="text-[12.5px] text-dim">HTTPS by default, pairing with a QR code + PIN, remote control off.</div>
        </div>
        <Switch checked={!!settings?.remote.enabled} onChange={(v) => void update({ remote: { enabled: v } }, { silent: true })} label="Turn on the phone companion" />
      </div>
    </div>,
  ];

  return (
    <motion.div className="fixed inset-0 z-[90] flex items-center justify-center p-6" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0, transition: { duration: 0.25 } }} role="dialog" aria-modal="true" aria-label="Welcome to OmniHub">
      <div className="absolute inset-0 bg-bg/80 backdrop-blur-xl" />
      <div className="app-backdrop" />
      <motion.div initial={{ opacity: 0, y: 20, scale: 0.97 }} animate={{ opacity: 1, y: 0, scale: 1 }} exit={{ opacity: 0, y: 10, scale: 0.98 }} transition={{ type: 'spring', stiffness: 260, damping: 28 }} className="relative w-full max-w-[720px] overflow-hidden rounded-[28px] border border-line-strong bg-elev shadow-[0_40px_120px_-30px_rgba(0,0,0,0.8)]">
        <div className="pointer-events-none absolute -right-24 -top-24 h-72 w-72 rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_70%)]" />
        <div className="relative min-h-[470px] overflow-hidden px-10 pb-6 pt-10">
          <AnimatePresence mode="wait" custom={dir}>
            <motion.div key={step} custom={dir} initial={{ opacity: 0, x: dir * 40 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: dir * -40 }} transition={{ type: 'spring', stiffness: 320, damping: 32 }}>
              {steps[step]}
            </motion.div>
          </AnimatePresence>
        </div>
        <div className="relative flex items-center gap-3 border-t border-line px-10 py-4">
          <div className="flex gap-1.5" aria-label={`Step ${step + 1} of ${total}`}>
            {Array.from({ length: total }, (_, i) => (
              <button key={i} type="button" aria-label={`Go to step ${i + 1}`} onClick={() => go(i)} className="h-2 rounded-full transition-all duration-300" style={{ width: i === step ? 22 : 8, background: i === step ? 'var(--accent)' : 'var(--surface-3)' }} />
            ))}
          </div>
          <div className="flex-1" />
          {step === 0 ? (
            <Button variant="ghost" onClick={() => void finish()}>
              Skip tour
            </Button>
          ) : (
            <Button variant="ghost" icon={ArrowLeft} onClick={() => go(step - 1)}>
              Back
            </Button>
          )}
          {step < total - 1 ? (
            <Button variant="primary" iconRight={ArrowRight} onClick={() => go(step + 1)} data-autofocus>
              {step === 0 ? 'Get started' : 'Next'}
            </Button>
          ) : (
            <Button variant="primary" icon={Check} onClick={() => void finish()}>
              Start using OmniHub
            </Button>
          )}
        </div>
      </motion.div>
    </motion.div>
  );
}
