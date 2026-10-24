// Chats: Claude and ChatGPT with your own API keys, and the Claude Code and
// Codex conversations on this PC, also from your phone; ideas saved from
// any message; and what each of them used.

import type { OpenAiSettings, Settings } from '@shared/types';
import { BarChart3, ExternalLink, KeyRound, Lightbulb, MessagesSquare, Smartphone, Sparkles } from 'lucide-react';
import { useEffect, useState } from 'react';
import { api, errorText } from '../../api';
import { Page } from '../../components/Page';
import { Button } from '../../components/ui/Button';
import { Field, Select, Switch, Tabs, TextInput } from '../../components/ui/Form';
import { Modal } from '../../components/ui/Overlay';
import { Callout } from '../../components/ui/States';
import { navigate, useRoute } from '../../lib/router';
import { toast } from '../../state/toasts';
import { ClaudeSettingsModal } from '../mods/shared';
import { ChatsTab } from './ChatsTab';
import { IdeasTab } from './IdeasTab';
import { CLAUDE_MODES, CODEX_SANDBOXES, KindIcon } from './shared';
import { UsageTab } from './UsageTab';

type Tab = 'chats' | 'ideas' | 'usage';

function KeysModal({ open, onClose, onClaude }: { open: boolean; onClose: () => void; onClaude: () => void }) {
  const [s, setS] = useState<OpenAiSettings | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [key, setKey] = useState('');
  const [chats, setChats] = useState<Settings['chats'] | null>(null);
  useEffect(() => {
    if (!open) return;
    void api.openai.settings().then((x) => {
      setS(x);
      if (x.hasKey) void api.openai.models().then(setModels, () => setModels([]));
    }, () => undefined);
    void api.app.settings().then((x) => setChats(x.chats), () => undefined);
  }, [open]);
  const update = async (patch: Partial<OpenAiSettings>) => {
    try {
      setS(await api.openai.update(patch));
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const saveKey = async (k: string) => {
    try {
      await api.openai.setKey(k);
      setKey('');
      const x = await api.openai.settings();
      setS(x);
      if (k) {
        const m = await api.openai.models();
        setModels(m);
        toast.success('Key saved', `It works: ${m.length} models available.`);
      } else {
        setModels([]);
        toast.info('Key removed');
      }
    } catch (e) {
      toast.error(k ? 'Key not accepted' : 'Not removed', errorText(e));
    }
  };
  const setChat = async (patch: Partial<Settings['chats']>) => {
    try {
      setChats((await api.app.updateSettings({ chats: patch })).chats);
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  const modelList = s && !models.includes(s.model) ? [s.model, ...models] : models;
  return (
    <Modal open={open} onClose={onClose} title="Keys and access" description="Your keys are kept encrypted for your Windows account, never shown again, and sent only to Anthropic or OpenAI." icon={KeyRound} size="md">
      <div className="space-y-5">
        <div className="flex items-center gap-3 rounded-xl border border-line px-4 py-3">
          <KindIcon s={{ source: 'omni', provider: 'claude' }} size={32} />
          <div className="min-w-0 flex-1">
            <div className="text-[13.5px] font-medium text-fg">Claude</div>
            <div className="text-[12px] text-dim">The same key and model as the Mods page’s mod maker.</div>
          </div>
          <Button size="sm" variant="secondary" icon={Sparkles} onClick={onClaude}>
            Claude settings
          </Button>
        </div>
        {s && (
          <div className="space-y-3 rounded-xl border border-line px-4 py-3">
            <div className="flex items-center gap-3">
              <KindIcon s={{ source: 'omni', provider: 'openai' }} size={32} />
              <div className="min-w-0 flex-1">
                <div className="text-[13.5px] font-medium text-fg">ChatGPT (OpenAI API)</div>
                <div className="text-[12px] text-dim">Billed to your OpenAI API account, separate from a ChatGPT subscription.</div>
              </div>
            </div>
            <Field label="API key" hint={s.hasKey ? 'A key is saved.' : 'Create one at platform.openai.com → API keys. It starts with sk-.'}>
              <div className="flex gap-2">
                <TextInput type="password" value={key} onChange={(e) => setKey(e.target.value)} placeholder={s.hasKey ? '•••••••• (saved)' : 'sk-…'} icon={KeyRound} className="flex-1" aria-label="OpenAI API key" autoComplete="off" />
                <Button size="sm" variant="primary" disabled={!key.trim()} onClick={() => void saveKey(key.trim())}>
                  Save
                </Button>
                {s.hasKey && (
                  <Button size="sm" variant="ghost" onClick={() => void saveKey('')}>
                    Remove
                  </Button>
                )}
              </div>
            </Field>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Model" hint={models.length ? undefined : 'The list fills in once your key is saved.'}>
                <Select value={s.model} onChange={(e) => void update({ model: e.target.value })}>
                  {modelList.map((m) => (
                    <option key={m} value={m}>
                      {m}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Reasoning">
                <Select value={s.effort} onChange={(e) => void update({ effort: e.target.value as OpenAiSettings['effort'] })}>
                  <option value="default">The model’s default</option>
                  <option value="low">Quick</option>
                  <option value="medium">Balanced</option>
                  <option value="high">Thorough</option>
                </Select>
              </Field>
            </div>
            <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>Let it search the web</span>
              <Switch checked={s.web} onChange={(v) => void update({ web: v })} label="Web search" />
            </label>
            <Button size="sm" variant="ghost" icon={ExternalLink} onClick={() => void api.app.openUrl('https://platform.openai.com/api-keys')}>
              Get a key
            </Button>
          </div>
        )}
        {chats && (
          <div className="space-y-3 rounded-xl border border-line px-4 py-3">
            <div className="flex items-center gap-2 text-[13.5px] font-medium text-fg">
              <Smartphone size={16} className="text-accent" /> From your phone
            </div>
            <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>
                Chats on paired phones
                <span className="block text-[11.5px] text-faint">Read every chat here, send messages and save ideas.</span>
              </span>
              <Switch checked={chats.allowPhone} onChange={(v) => void setChat({ allowPhone: v })} label="Chats on paired phones" />
            </label>
            <label className="flex items-center justify-between gap-3 text-[13px] text-fg">
              <span>
                Run Claude Code and Codex from the phone
                <span className="block text-[11.5px] text-faint">Only in folders they already work in, and never with permissions skipped.</span>
              </span>
              <Switch checked={chats.phoneAgents} disabled={!chats.allowPhone} onChange={(v) => void setChat({ phoneAgents: v })} label="Run agents from the phone" />
            </label>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Claude Code may" hint={CLAUDE_MODES.find((m) => m.id === chats.claudeMode)?.hint}>
                <Select value={chats.claudeMode} onChange={(e) => void setChat({ claudeMode: e.target.value as Settings['chats']['claudeMode'] })}>
                  {CLAUDE_MODES.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                </Select>
              </Field>
              <Field label="Codex may" hint={CODEX_SANDBOXES.find((m) => m.id === chats.codexSandbox)?.hint}>
                <Select value={chats.codexSandbox} onChange={(e) => void setChat({ codexSandbox: e.target.value as Settings['chats']['codexSandbox'] })}>
                  {CODEX_SANDBOXES.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                </Select>
              </Field>
            </div>
          </div>
        )}
        <Callout tone="info">Claude Code and Codex use your own sign-in on this PC (your Claude or ChatGPT plan); OmniHub runs them for you and needs no key for them.</Callout>
      </div>
    </Modal>
  );
}

export function ChatsPage() {
  const route = useRoute();
  const tab = (route.params.get('tab') as Tab | null) ?? 'chats';
  const [keys, setKeys] = useState(false);
  const [claude, setClaude] = useState(false);
  const setTab = (t: Tab) => navigate('chats', t === 'chats' ? {} : { tab: t });
  const idea = async (text: string, from: string) => {
    try {
      await api.ideas.save(text, from);
      toast.success('Saved as an idea', 'Find it under Ideas, and in Notes.');
    } catch (e) {
      toast.error('Not saved', errorText(e));
    }
  };
  return (
    <Page
      title="Chats"
      subtitle="Claude, ChatGPT, Claude Code and Codex in one place, and on your phone. Save anything as an idea, and see what you’ve used."
      actions={
        <Button variant="secondary" icon={KeyRound} onClick={() => setKeys(true)}>
          Keys and access
        </Button>
      }
      headerExtra={
        <Tabs
          value={tab}
          onChange={setTab}
          items={[
            { value: 'chats', label: 'Chats', icon: MessagesSquare },
            { value: 'ideas', label: 'Ideas', icon: Lightbulb },
            { value: 'usage', label: 'Usage', icon: BarChart3 },
          ]}
        />
      }
    >
      <KeysModal open={keys} onClose={() => setKeys(false)} onClaude={() => (setKeys(false), setClaude(true))} />
      <ClaudeSettingsModal open={claude} onClose={() => setClaude(false)} />
      {tab === 'chats' && <ChatsTab onIdea={(t, f) => void idea(t, f)} />}
      {tab === 'ideas' && <IdeasTab />}
      {tab === 'usage' && <UsageTab />}
    </Page>
  );
}
