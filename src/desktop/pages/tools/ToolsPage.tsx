// Tools: file tools, ports and window layouts.

import { CloudDownload, FileCog, Film, FolderInput, GitBranch, Image, LayoutDashboard, Network, PenTool, Terminal } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Page } from '../../components/Page';
import { type TabItem, Tabs } from '../../components/ui/Form';
import { useRoute } from '../../lib/router';
import { BoardTab } from './BoardTab';
import { DownloadsTab } from './DownloadsTab';
import { EnvTab } from './EnvTab';
import { FilesTab } from './FilesTab';
import { ImagesTab } from './ImagesTab';
import { LayoutsTab } from './LayoutsTab';
import { PortsTab } from './PortsTab';
import { ProjectsTab } from './ProjectsTab';
import { RulesTab } from './RulesTab';
import { VideoTab } from './VideoTab';

type Tab = 'files' | 'images' | 'video' | 'downloads' | 'board' | 'rules' | 'ports' | 'layouts' | 'projects' | 'env';
const TABS: TabItem<Tab>[] = [
  { value: 'files', label: 'Files', icon: FileCog },
  { value: 'images', label: 'Images', icon: Image },
  { value: 'video', label: 'Video', icon: Film },
  { value: 'downloads', label: 'Downloads', icon: CloudDownload },
  { value: 'board', label: 'Whiteboard', icon: PenTool },
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
    <Page title="Tools" subtitle="Files, images, video, downloads, the phone’s whiteboard, folder rules, layouts, ports, Git projects and PATH." scroll={tab !== 'ports'}>
      <Tabs items={TABS} value={tab} onChange={setTab} className="mb-4" />
      {tab === 'files' && <FilesTab />}
      {tab === 'images' && <ImagesTab />}
      {tab === 'video' && <VideoTab />}
      {tab === 'downloads' && <DownloadsTab />}
      {tab === 'board' && <BoardTab />}
      {tab === 'ports' && <PortsTab />}
      {tab === 'layouts' && <LayoutsTab />}
      {tab === 'rules' && <RulesTab />}
      {tab === 'projects' && <ProjectsTab />}
      {tab === 'env' && <EnvTab />}
    </Page>
  );
}
