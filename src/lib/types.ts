export interface Workspace {
  id: string;
  name: string;
  path: string;
  agent: string | null;
  notes: string | null;
  icon: string | null;
  color: string | null;
  groupId: string | null;
  isFavorite: boolean;
  createdAt: number;
  lastOpenedAt: number | null;
  tags: string[];
  launchConfigs: LaunchConfig[];
}

export interface LaunchConfig {
  id: string;
  workspaceId: string;
  name: string;
  command: string;
  shell: string;
  confirmSkip: boolean;
}

export interface LaunchConfigInput {
  name: string;
  command: string;
  shell: string;
  confirmSkip: boolean;
}

export interface WorkspaceInput {
  name: string;
  path: string;
  agent: string | null;
  notes: string | null;
  icon: string | null;
  color: string | null;
  groupId: string | null;
  launchConfigs: LaunchConfigInput[];
  tags: string[];
}

export interface Group {
  id: string;
  name: string;
  icon: string | null;
  sortOrder: number;
}

export interface NewGroup {
  name: string;
  icon: string | null;
}

export interface CandidateDir {
  path: string;
  dirName: string;
  markers: string[];
}

export interface FeedItem {
  ts: number;
  kind: "launch" | "open" | "work" | "commit" | "snapshot";
  wsId: string;
  wsName: string;
  agent: string | null;
  text: string;
}

export interface SnapInfo {
  file: string;
  createdAt: number;
  size: number;
}

export interface FsEntry {
  name: string;
  isDir: boolean;
  size: number;
  modified: number;
}

export interface AgentInfo {
  id: string;
  name: string;
  command: string;
}

export interface QuickLaunchInput {
  workspaceId: string;
  name: string;
  command: string;
  shell: string;
  confirmSkip: boolean;
}

export interface GitStatus {
  isRepo: boolean;
  branch: string | null;
  dirty: number;
}

export interface FailedImport {
  path: string;
  reason: string;
}

export interface ImportSummary {
  added: Workspace[];
  failed: FailedImport[];
}
