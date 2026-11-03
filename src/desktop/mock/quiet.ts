// Quiet mode for the mock backend.

import type { QuietStatus } from '@shared/types';
import { emit } from './bus';

let on: number | null = null;
export const quiet = {
  status: (): QuietStatus => ({ on: on != null, since: on, fans: true }),
  set: (v: boolean): QuietStatus => {
    on = v ? Date.now() : null;
    emit('quiet:changed', quiet.status());
    return quiet.status();
  },
};
