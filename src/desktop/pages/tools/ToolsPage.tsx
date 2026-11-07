// Tools: file tools, ports and window layouts.

import { Archive, CloudDownload, FileCog, Film, FolderInput, Gamepad, GitBranch, Image, LayoutDashboard, MousePointerClick, Network, PenTool, Terminal, AlarmClock, Stethoscope } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Page } from '../../components/Page';
import { type TabItem, Tabs } from '../../components/ui/Form';
import { useRoute } from '../../lib/router';
import { AutoClickerTab } from './AutoClickerTab';
import { BackupsTab } from './BackupsTab';
import { BoardTab } from './BoardTab';
import { DownloadsTab } from './DownloadsTab';
import { EnvTab } from './EnvTab';
import { FilesTab } from './FilesTab';
import { ImagesTab } from './ImagesTab';
import { LayoutsTab } from './LayoutsTab';
import { PokemonSaveTab } from './PokemonSaveTab';
import { PortsTab } from './PortsTab';
import { ProjectsTab } from './ProjectsTab';
import { RulesTab } from './RulesTab';
import { VideoTab } from './VideoTab';
import { ChecksTab } from './ChecksTab';
import { TimersTab } from './TimersTab';

type Tab = 'timers' | 'checks' | 'files' | 'images' | 'video' | 'downloads' | 'board' | 'clicker' | 'pokemon' | 'rules' | 'backups' | 'ports' | 'layouts' | 'projects' | 'env';
const TABS: TabItem<Tab>[] = [
  { value: 'timers', label: 'Timers', icon: AlarmClock },
  { value: 'checks', label: 'PC checks', icon: Stethoscope },
  { value: 'files', label: 'Files', icon: FileCog },
  { value: 'images', label: 'Images', icon: Image },
  { value: 'video', label: 'Video', icon: Film },
  { value: 'downloads', label: 'Downloads', icon: CloudDownload },
  { value: 'board', label: 'Whiteboard', icon: PenTool },
  { value: 'clicker', label: 'Auto clicker', icon: MousePointerClick },
  { value: 'pokemon', label: 'Pokémon saves', icon: Gamepad },
  { value: 'rules', label: 'Folder rules', icon: FolderInput },
  { value: 'backups', label: 'Backups', icon: Archive },
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
    <Page title="Tools" subtitle="Timers and bedtime, PC checks, files, images, video, downloads, the phone’s whiteboard, an auto clicker, a Pokémon save editor, folder rules, backups, layouts, ports, Git projects and PATH." scroll={tab !== 'ports'}>
      <Tabs items={TABS} value={tab} onChange={setTab} className="mb-4" />
      {tab === 'timers' && <TimersTab />}
      {tab === 'checks' && <ChecksTab />}
      {tab === 'files' && <FilesTab />}
      {tab === 'images' && <ImagesTab />}
      {tab === 'video' && <VideoTab />}
      {tab === 'downloads' && <DownloadsTab />}
      {tab === 'board' && <BoardTab />}
      {tab === 'clicker' && <AutoClickerTab />}
      {tab === 'pokemon' && <PokemonSaveTab />}
      {tab === 'ports' && <PortsTab />}
      {tab === 'layouts' && <LayoutsTab />}
      {tab === 'rules' && <RulesTab />}
      {tab === 'backups' && <BackupsTab />}
      {tab === 'projects' && <ProjectsTab />}
      {tab === 'env' && <EnvTab />}
    </Page>
  );
}
