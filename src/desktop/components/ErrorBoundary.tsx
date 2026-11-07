// A page that throws shows this instead of a blank window, and the error is
// written to OmniHub's log so it can be reported.

import { Component, type ErrorInfo, type ReactNode } from 'react';
import { api } from '../api';
import { Button } from './ui/Button';

interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<{ children: ReactNode; name: string }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    void api.app.logError(`${this.props.name}: ${error.stack || error.message}\n${info.componentStack ?? ''}`).catch(() => undefined);
  }

  render() {
    const { error } = this.state;
    if (!error) return this.props.children;
    const details = `${this.props.name}: ${error.message}\n${error.stack ?? ''}`;
    return (
      <div className="flex h-full items-center justify-center p-8">
        <div className="max-w-[520px] rounded-2xl border border-line bg-surface p-6">
          <h2 className="font-display text-[20px] font-semibold text-fg">This page hit a problem</h2>
          <p className="mt-2 text-[13px] text-dim">The rest of OmniHub keeps working. The error was written to OmniHub's log; copy it if you report the problem.</p>
          <pre className="mt-3 max-h-40 overflow-auto whitespace-pre-wrap rounded-lg bg-surface-2 p-3 font-mono text-[11px] text-faint">{error.message}</pre>
          <div className="mt-4 flex gap-2">
            <Button variant="primary" onClick={() => this.setState({ error: null })}>
              Try again
            </Button>
            <Button variant="ghost" onClick={() => void navigator.clipboard?.writeText(details)}>
              Copy details
            </Button>
          </div>
        </div>
      </div>
    );
  }
}
