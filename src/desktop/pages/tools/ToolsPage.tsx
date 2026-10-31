// Tools: file tools, ports and window layouts.

import { FileCog, FolderInput, GitBranch, LayoutDashboard, Network, Terminal } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Page } from '../../components/Page';
import { type TabItem, Tabs } from '../../components/ui/Form';
import { useRoute } from '../../lib/router';
import { EnvTab } from './EnvTab';
import { FilesTab } from './FilesTab';
import { LayoutsTab } from './LayoutsTab';
import { PortsTab } from './PortsTab';
import { ProjectsTab } from './ProjectsTab';
import { RulesTab } from './RulesTab';

type Tab = 'files' | 'rules' | 'ports' | 'layouts' | 'projects' | 'env';
const TABS: TabItem<Tab>[] = [
  { value: 'files', label: 'Files', icon: FileCog },
  { value: 'rules', label: 'Folder rules', icon: FolderInput },
  { value: 'layouts', label: 'Window layouts', icon: LayoutDashboard },
  { value: 'ports', label: 'Ports', icon: Network },
  { value: 'projects', label: 'Projects', icon: GitBranch },
  { value: 'env', label: 'PATH & variables', icon: Terminal },
];

export function ToolsPage() {
  const route = useRoute();
  const [tab, setTab] = useState<Tab>('files');
  useEffect(() => {
    const t = route.params.get('tab') as Tab | null;
    if (t && TABS.some((x) => x.value === t)) setTab(t);
  }, [route.params]);
  return (
    <Page title="Tools" subtitle="Files, folder rules, window layouts, ports, your Git projects and PATH." scroll={tab !== 'ports'}>
      <Tabs items={TABS} value={tab} onChange={setTab} className="mb-4" />
      {tab === 'files' && <FilesTab />}
      {tab === 'ports' && <PortsTab />}
      {tab === 'layouts' && <LayoutsTab />}
      {tab === 'rules' && <RulesTab />}
      {tab === 'projects' && <ProjectsTab />}
      {tab === 'env' && <EnvTab />}
    </Page>
  );
}
