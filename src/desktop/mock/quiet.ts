// Quiet mode for the mock backend.

import type { QuietSchedule, QuietStatus } from '@shared/types';
import { emit } from './bus';

let on: number | null = null;
let timer: number | null = null;
let schedule: QuietSchedule = { enabled: false, start: '23:00', end: '07:30' };
export const quiet = {
  status: (): QuietStatus => ({ schedule, timer, on: on != null, since: on, fans: true }),
  set: (v: boolean): QuietStatus => {
    on = v ? Date.now() : null;
    timer = null;
    emit('quiet:changed', quiet.status());
    return quiet.status();
  },
  setSchedule: (s: QuietSchedule): QuietStatus => {
    if (!/^\d{1,2}:\d{2}$/.test(s.start) || !/^\d{1,2}:\d{2}$/.test(s.end)) throw new Error('Times look like 23:00');
    schedule = s;
    emit('quiet:changed', quiet.status());
    return quiet.status();
  },
  setTimer: (m: number | null): QuietStatus => {
    timer = m == null ? null : Date.now() + m * 60_000;
    emit('quiet:changed', quiet.status());
    return quiet.status();
  },
};
