// Tools: file tools, ports and window layouts.

import { FileCog, LayoutDashboard, Network } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Page } from '../../components/Page';
import { type TabItem, Tabs } from '../../components/ui/Form';
import { useRoute } from '../../lib/router';
import { FilesTab } from './FilesTab';
import { LayoutsTab } from './LayoutsTab';
import { PortsTab } from './PortsTab';

type Tab = 'files' | 'ports' | 'layouts';
const TABS: TabItem<Tab>[] = [
  { value: 'files', label: 'Files', icon: FileCog },
  { value: 'ports', label: 'Ports', icon: Network },
  { value: 'layouts', label: 'Window layouts', icon: LayoutDashboard },
];

export function ToolsPage() {
  const route = useRoute();
  const [tab, setTab] = useState<Tab>('files');
  useEffect(() => {
    const t = route.params.get('tab') as Tab | null;
    if (t && TABS.some((x) => x.value === t)) setTab(t);
  }, [route.params]);
  return (
    <Page title="Tools" subtitle="Checksums, shredding, bulk rename, recent files, ports and window layouts." scroll={tab !== 'ports'}>
      <Tabs items={TABS} value={tab} onChange={setTab} className="mb-4" />
      {tab === 'files' && <FilesTab />}
      {tab === 'ports' && <PortsTab />}
      {tab === 'layouts' && <LayoutsTab />}
    </Page>
  );
}
