import type { LucideIcon } from 'lucide-react';
import { Activity, Camera, Timer, Wrench, Cpu, Gamepad2, HardDrive, House, LayoutGrid, Lightbulb, LockKeyhole, MessagesSquare, Music2, NotebookPen, Puzzle, ScreenShare, Server, Settings, Smartphone } from 'lucide-react';
import type { RouteId } from '../lib/router';

export interface NavItem {
  id: Exclude<RouteId, 'overlay'>;
  label: string;
  icon: LucideIcon;
  description: string;
  group: 'workspace' | 'performance' | 'security' | 'devices' | 'app';
}

export const NAV: NavItem[] = [
  { id: 'home', label: 'Home', icon: House, description: 'Your PC at a glance', group: 'workspace' },
  { id: 'storage', label: 'Storage', icon: HardDrive, description: 'See what fills your drives and clean up', group: 'workspace' },
  { id: 'apps', label: 'Apps', icon: LayoutGrid, description: 'Installed programs and Store apps', group: 'workspace' },
  { id: 'screenshots', label: 'Screenshots', icon: Camera, description: 'Capture, tag and find screenshots', group: 'workspace' },
  { id: 'notes', label: 'Notes', icon: NotebookPen, description: 'Notes and ideas for Claude', group: 'workspace' },
  { id: 'chats', label: 'Chats', icon: MessagesSquare, description: 'Claude, ChatGPT, Claude Code and Codex in one place, also from your phone; ideas and usage', group: 'workspace' },
  { id: 'focus', label: 'Focus', icon: Timer, description: 'Focus timer that keeps distractions closed, and where your time goes', group: 'workspace' },
  { id: 'music', label: 'Music', icon: Music2, description: 'What your PC plays, with full-screen lyrics', group: 'workspace' },
  { id: 'tasks', label: 'Tasks', icon: Activity, description: 'CPU, memory, GPU and disk use of every program', group: 'performance' },
  { id: 'tools', label: 'Tools', icon: Wrench, description: 'Checksums, shredder, bulk rename, folder rules, window layouts, ports, Git projects, PATH', group: 'performance' },
  { id: 'games', label: 'Games', icon: Gamepad2, description: 'Game boosts, Fortnite and PC optimizations, ping helper', group: 'performance' },
  { id: 'mods', label: 'Mods', icon: Puzzle, description: 'Mods for your Steam games, Minecraft modpack mixing, and Claude making mods', group: 'performance' },
  { id: 'servers', label: 'Servers', icon: Server, description: 'Host Minecraft servers 24/7 on this PC: players, live map, mods, plugins, modpacks and backups', group: 'performance' },
  { id: 'rig', label: 'My PC', icon: Cpu, description: 'Your PC in 3D: lighting, fan profiles and screens', group: 'performance' },
  { id: 'vault', label: 'Vault', icon: LockKeyhole, description: 'Encrypted passwords and secrets', group: 'security' },
  { id: 'phone', label: 'Phone', icon: Smartphone, description: 'Companion server, pairing and transfers', group: 'devices' },
  { id: 'lights', label: 'Lights', icon: Lightbulb, description: 'Govee lights: colours, screen and music sync, presets and timers', group: 'devices' },
  { id: 'screen', label: 'Screen share', icon: ScreenShare, description: 'Stream your PC or mirror an Android phone', group: 'devices' },
  { id: 'settings', label: 'Settings', icon: Settings, description: 'Appearance, privacy and behaviour', group: 'app' },
];
