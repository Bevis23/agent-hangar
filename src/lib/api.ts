import { invoke } from "@tauri-apps/api/core";
import type {
  AgentInfo,
  CandidateDir,
  FeedItem,
  FsEntry,
  GitStatus,
  Group,
  ImportSummary,
  NewGroup,
  QuickLaunchInput,
  SnapInfo,
  Workspace,
  WorkspaceInput,
} from "./types";

export const api = {
  list: () => invoke<Workspace[]>("list_workspaces"),
  add: (payload: WorkspaceInput) => invoke<Workspace>("add_workspace", { payload }),
  update: (payload: WorkspaceInput & { id: string }) =>
    invoke<Workspace>("update_workspace", { payload }),
  setFavorite: (id: string, favorite: boolean) =>
    invoke<void>("set_favorite", { id, favorite }),
  remove: (id: string) => invoke<void>("delete_workspace", { id }),
  recordOpen: (id: string, via: string) => invoke<void>("record_open", { id, via }),
  openInExplorer: (path: string) => invoke<void>("open_in_explorer", { path }),
  openInTerminal: (path: string) => invoke<void>("open_in_terminal", { path }),

  listGroups: () => invoke<Group[]>("list_groups"),
  createGroup: (payload: NewGroup) => invoke<Group>("create_group", { payload }),
  deleteGroup: (id: string) => invoke<void>("delete_group", { id }),
  setWorkspaceGroup: (id: string, group: string | null) =>
    invoke<void>("set_workspace_group", { id, group }),

  scanImportRoot: (path: string, all: boolean) =>
    invoke<CandidateDir[]>("scan_import_root", { path, all }),
  importWorkspaces: (payloads: WorkspaceInput[]) =>
    invoke<ImportSummary>("import_workspaces", { payloads }),

  launch: (config: string) => invoke<number>("launch", { config }),
  processStatuses: () => invoke<Record<string, string>>("process_statuses"),
  stopProcess: (config: string) => invoke<void>("stop_process", { config }),
  setLaunchConfirmSkip: (config: string, skip: boolean) =>
    invoke<void>("set_launch_confirm_skip", { config, skip }),
  gitStatuses: (paths: string[]) => invoke<Record<string, GitStatus>>("git_statuses", { paths }),
  getSettings: () => invoke<Record<string, string>>("get_settings"),
  setSettings: (entries: Record<string, string>) =>
    invoke<void>("set_settings", { entries }),
  openInEditor: (path: string) => invoke<void>("open_in_editor", { path }),
  quickLaunch: (payload: QuickLaunchInput) => invoke<number>("quick_launch", { payload }),
  listInstalledAgents: () => invoke<AgentInfo[]>("list_installed_agents"),
  listDir: (path: string) => invoke<FsEntry[]>("list_dir", { path }),
  openEntry: (path: string) => invoke<void>("open_entry", { path }),
  readTextFile: (path: string) => invoke<{ content: string; truncated: boolean; size: number }>("read_text_file", { path }),
  writeTextFile: (path: string, content: string) => invoke<void>("write_text_file", { path, content }),
  listSnapshots: (workspaceId: string) => invoke<SnapInfo[]>("list_snapshots", { workspaceId }),
  restoreSnapshot: (workspaceId: string, file: string, path: string) =>
    invoke<number>("restore_snapshot", { workspaceId, file, path }),
  deleteSnapshot: (workspaceId: string, file: string) =>
    invoke<void>("delete_snapshot", { workspaceId, file }),
  activityFeed: (days: number) => invoke<FeedItem[]>("activity_feed", { days }),
  getAutostart: () => invoke<boolean>("get_autostart"),
  setAutostart: (enable: boolean) => invoke<void>("set_autostart", { enable }),
};
