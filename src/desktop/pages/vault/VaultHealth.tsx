import type { HealthItem, HealthReport } from '@shared/types';
import type { LucideIcon } from 'lucide-react';
import { CalendarClock, Copy, KeyRound, ShieldAlert, ShieldCheck, Siren, Smartphone } from 'lucide-react';
import { type ReactNode, useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Button } from '../../components/ui/Button';
import { Badge, Skeleton } from '../../components/ui/Card';
import { Modal } from '../../components/ui/Overlay';
import { ProgressRing } from '../../components/ui/Progress';
import { Callout } from '../../components/ui/States';
import { cx } from '../../lib/cx';
import { prettyUrl } from '../../lib/util';
import { toast } from '../../state/toasts';

function Section({ icon: Icon, tone, title, hint, children, count }: { icon: LucideIcon; tone: 'bad' | 'warn' | 'info'; title: string; hint: string; count: number; children: ReactNode }) {
  if (!count) return null;
  return (
    <section>
      <div className="mb-1.5 flex items-center gap-2">
        <Icon size={15} className={tone === 'bad' ? 'text-bad' : tone === 'warn' ? 'text-warn' : 'text-info'} aria-hidden />
        <h3 className="text-[13.5px] font-semibold text-fg">{title}</h3>
        <Badge tone={tone}>{count}</Badge>
      </div>
      <p className="mb-2 text-[12px] text-faint">{hint}</p>
      <div className="space-y-1">{children}</div>
    </section>
  );
}

function Row({ i, right, onOpen }: { i: HealthItem; right?: ReactNode; onOpen: (id: string) => void }) {
  return (
    <button type="button" onClick={() => onOpen(i.id)} className="flex w-full items-center gap-3 rounded-xl border border-line bg-surface px-3 py-2 text-left transition-colors hover:border-accent/40 hover:bg-surface-2">
      <KeyRound size={14} className="shrink-0 text-faint" aria-hidden />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[13px] font-medium text-fg">{i.title}</span>
        <span className="block truncate text-[11.5px] text-faint">{[i.username, prettyUrl(i.url)].filter(Boolean).join(' · ')}</span>
      </span>
      {right}
    </button>
  );
}

/** How healthy the vault's passwords are, with the entries to fix. */
export function VaultHealth({ open, onClose, onOpenEntry }: { open: boolean; onClose: () => void; onOpenEntry: (id: string) => void }) {
  const [report, setReport] = useState<HealthReport | null>(null);
  const [breached, setBreached] = useState<HealthItem[] | null>(null);
  const [checking, setChecking] = useState(false);
  useEffect(() => {
    if (!open) return;
    setReport(null);
    api.vault.health().then(setReport, (e: unknown) => toast.error('Could not check the vault', errorText(e)));
  }, [open]);
  const go = (id: string) => {
    onClose();
    onOpenEntry(id);
  };
  const check = async () => {
    setChecking(true);
    try {
      setBreached(await api.vault.breachCheck());
    } catch (e) {
      toast.error('The breach check failed', errorText(e));
    } finally {
      setChecking(false);
    }
  };
  const tone = !report ? undefined : report.score >= 80 ? 'var(--good)' : report.score >= 50 ? 'var(--warn)' : 'var(--bad)';
  const problems = report ? report.weak.length + report.reused.length + report.old.length + report.missingTwoFactor.length + (breached?.length ?? 0) : 0;
  return (
    <Modal open={open} onClose={onClose} title="Password health" description="Checked on this PC. Click an entry to fix it." icon={ShieldCheck} size="lg">
      {!report ? (
        <div className="space-y-3 pb-2">
          <Skeleton className="h-24 rounded-2xl" />
          <Skeleton className="h-40 rounded-2xl" />
        </div>
      ) : (
        <div className="space-y-5 pb-2">
          <div className="flex items-center gap-4 rounded-2xl border border-line bg-surface px-5 py-4">
            <ProgressRing value={report.score / 100} size={72} stroke={7} color={tone} label="Health score">
              <span className="font-display text-[19px] font-semibold tabular text-fg">{report.score}</span>
            </ProgressRing>
            <div className="min-w-0 flex-1">
              <div className="text-[15px] font-semibold text-fg">{report.score >= 80 ? 'Looking good' : report.score >= 50 ? 'Worth a tidy-up' : 'Needs attention'}</div>
              <div className="text-[12.5px] text-dim">
                {report.checked} passwords checked · {problems ? `${problems} thing${problems > 1 ? 's' : ''} to improve` : 'nothing to fix'}
              </div>
            </div>
          </div>

          <Section icon={ShieldAlert} tone="bad" title="Weak passwords" hint="Short or common — easy to guess. Generate a strong one in the entry." count={report.weak.length}>
            {report.weak.map((i) => (
              <Row key={i.id} i={i} onOpen={go} right={<Badge tone="bad">{['very weak', 'weak'][i.detail] ?? 'weak'}</Badge>} />
            ))}
          </Section>
          <Section icon={Copy} tone="bad" title="Reused passwords" hint="One leak opens every account that shares it. Give each a different password." count={report.reused.length}>
            {report.reused.map((g, n) => (
              <div key={n} className="space-y-1 rounded-xl border border-dashed border-bad/30 p-1.5">
                <div className="px-1.5 text-[11px] text-faint">Same password on {g.length} entries</div>
                {g.map((i) => (
                  <Row key={i.id} i={i} onOpen={go} />
                ))}
              </div>
            ))}
          </Section>
          <Section icon={CalendarClock} tone="warn" title="Not changed in over a year" hint="Older passwords are more likely to have leaked somewhere." count={report.old.length}>
            {report.old.map((i) => (
              <Row key={i.id} i={i} onOpen={go} right={<span className="text-[11.5px] text-faint tabular">{Math.floor(i.detail / 365)}+ years</span>} />
            ))}
          </Section>
          <Section icon={Smartphone} tone="info" title="Could use two-factor codes" hint="These sites offer authenticator codes. Turn it on there, then paste the setup key into the entry's 2FA field." count={report.missingTwoFactor.length}>
            {report.missingTwoFactor.map((i) => (
              <Row key={i.id} i={i} onOpen={go} />
            ))}
          </Section>

          <div className={cx('rounded-2xl border p-4', breached?.length ? 'border-bad/40 bg-bad/8' : 'border-line bg-surface')}>
            <div className="flex items-start gap-3">
              <Siren size={18} className={breached?.length ? 'text-bad' : 'text-faint'} aria-hidden />
              <div className="min-w-0 flex-1">
                <div className="text-[13.5px] font-semibold text-fg">Known data breaches</div>
                <p className="mt-0.5 text-[12px] leading-relaxed text-faint">
                  Checks your passwords against Have I Been Pwned. Only the first 5 characters of each password's SHA-1 hash are sent; the match happens on this PC.
                </p>
              </div>
              <Button size="sm" variant={breached ? 'secondary' : 'primary'} loading={checking} onClick={() => void check()}>
                {breached ? 'Check again' : 'Check now'}
              </Button>
            </div>
            {breached && (
              <div className="mt-3 space-y-1">
                {breached.length === 0 ? (
                  <Callout tone="good" icon={ShieldCheck} title="None of your passwords appear in known breaches" />
                ) : (
                  breached.map((i) => <Row key={i.id} i={i} onOpen={go} right={<Badge tone="bad">seen {i.detail.toLocaleString()}×</Badge>} />)
                )}
              </div>
            )}
          </div>
        </div>
      )}
    </Modal>
  );
}
