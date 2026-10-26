import { computed, reactive, ref, watch } from "vue";
import { api } from "../lib/api";
import type { GitStatus, Group, NewGroup, Workspace } from "../lib/types";
import { BG_THEMES, DEFAULT_SETTINGS } from "../lib/utils";

/** 分组过滤值："" = 全部，"none" = 未分组，其余为 group.id */
export const GROUP_ALL = "";
export const GROUP_NONE = "none";

export function useWorkspaces() {
  const workspaces = ref<Workspace[]>([]);
  const groups = ref<Group[]>([]);
  const activeGroup = ref<string>(GROUP_ALL);
  const search = ref("");
  const loading = ref(false);
  const error = ref<string | null>(null);
  /** 启动配置 id → "running"；不在表中的视为未运行/未知 */
  const statuses = ref<Record<string, string>>({});
  /** path → git 状态（分支 + 未提交数），后台每 30s 刷新 */
  const gitStatuses = ref<Record<string, GitStatus>>({});
  const agentFilter = ref("");
  const gitFilter = ref("");

  /** 应用设置：合并默认值；改动即应用并持久化 */
  const settings = reactive<Record<string, string>>({ ...DEFAULT_SETTINGS });
  let settingsLoaded = false;

  function applySettings(s: Record<string, string>) {
    const root = document.documentElement;
    const theme = BG_THEMES[s.bg_theme] ?? BG_THEMES.midnight;
    root.style.setProperty("--color-hangar-bg", theme.bg);
    root.style.setProperty("--color-hangar-panel", theme.panel);
    root.style.setProperty("--color-hangar-border", theme.border);
    root.style.setProperty("--color-hangar-accent", s.accent || "#f5a524");
    document.body.style.background = theme.bg;
  }

  watch(
    settings,
    (s) => {
      applySettings(s);
      if (settingsLoaded) {
        api.setSettings({ ...s }).catch(() => void 0); // 保存失败不打断
      }
    },
    { deep: true },
  );

  async function loadSettings() {
    try {
      const saved = await api.getSettings();
      Object.assign(settings, DEFAULT_SETTINGS, saved);
    } catch {
      Object.assign(settings, DEFAULT_SETTINGS);
    } finally {
      settingsLoaded = true;
      applySettings({ ...settings });
    }
  }

  function resetSettings() {
    Object.assign(settings, DEFAULT_SETTINGS);
  }

  async function openEditor(w: Workspace) {
    try {
      await api.openInEditor(w.path);
      await api.recordOpen(w.id, "editor");
      await load();
    } catch (e) {
      error.value = String(e);
    }
  }

  /** 卡片「启动…」菜单：按名称 upsert 一条启动配置并立即运行。终端由后端智能选择。 */
  async function quickLaunch(workspaceId: string, name: string, command: string, remember: boolean) {
    try {
      await api.quickLaunch({
        workspaceId,
        name,
        command,
        shell: "gitbash",
        confirmSkip: remember,
      });
      await Promise.all([load(), pollStatuses()]);
    } catch (e) {
      error.value = String(e);
    }
  }

  let pollTimer: number | undefined;
  let gitTimer: number | undefined;

  async function refreshGit() {
    const paths = workspaces.value.map((w) => w.path);
    if (paths.length === 0) return;
    try {
      gitStatuses.value = await api.gitStatuses(paths);
    } catch {
      /* 刷新失败静默，下一轮重试 */
    }
  }

  async function pollStatuses() {
    try {
      statuses.value = await api.processStatuses();
    } catch {
      /* 轮询失败静默，下一轮重试 */
    }
  }

  function startPolling() {
    if (pollTimer) return;
    pollStatuses();
    pollTimer = window.setInterval(pollStatuses, 2000);
    refreshGit();
    gitTimer = window.setInterval(refreshGit, 30000);
  }
  function stopPolling() {
    if (pollTimer) window.clearInterval(pollTimer);
    pollTimer = undefined;
    if (gitTimer) window.clearInterval(gitTimer);
    gitTimer = undefined;
  }

  async function launch(configId: string) {
    try {
      await api.launch(configId);
      await pollStatuses();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function stopConfig(configId: string) {
    try {
      await api.stopProcess(configId);
      await pollStatuses();
    } catch (e) {
      error.value = String(e);
    }
  }

  /** 确认弹窗通过后的启动入口；勾了"记住"则先持久化再启动。 */
  async function confirmAndLaunch(configId: string, remember: boolean) {
    if (remember) {
      try {
        await api.setLaunchConfirmSkip(configId, true);
      } catch {
        /* 持久化失败不阻塞启动 */
      }
    }
    await launch(configId);
  }

  async function load() {
    loading.value = true;
    error.value = null;
    try {
      const [ws, gs] = await Promise.all([api.list(), api.listGroups()]);
      workspaces.value = ws;
      groups.value = gs;
      // 当前过滤的分组被删除时回退到"全部"
      if (
        activeGroup.value !== GROUP_ALL &&
        activeGroup.value !== GROUP_NONE &&
        !gs.some((g) => g.id === activeGroup.value)
      ) {
        activeGroup.value = GROUP_ALL;
      }
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  const filtered = computed(() => {
    let list = workspaces.value;
    if (activeGroup.value === GROUP_NONE) {
      list = list.filter((w) => !w.groupId);
    } else if (activeGroup.value !== GROUP_ALL) {
      list = list.filter((w) => w.groupId === activeGroup.value);
    }
    if (agentFilter.value) {
      list = list.filter((w) => w.agent === agentFilter.value);
    }
    if (gitFilter.value) {
      list = list.filter((w) => {
        const st = gitStatuses.value[w.path];
        if (!st) return false;
        if (gitFilter.value === "dirty") return st.isRepo && st.dirty > 0;
        if (gitFilter.value === "clean") return st.isRepo && st.dirty === 0;
        if (gitFilter.value === "nonrepo") return !st.isRepo;
        return true;
      });
    }
    const q = search.value.trim().toLowerCase();
    if (q) {
      list = list.filter((w) =>
        [w.name, w.path, w.agent ?? "", w.notes ?? "", ...w.tags]
          .join("\n")
          .toLowerCase()
          .includes(q),
      );
    }
    return list;
  });

  const groupCounts = computed(() => {
    const counts = new Map<string, number>();
    for (const w of workspaces.value) {
      const key = w.groupId ?? GROUP_NONE;
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return counts;
  });

  async function run<T>(fn: () => Promise<T>): Promise<boolean> {
    try {
      await fn();
      await load();
      return true;
    } catch (e) {
      error.value = String(e);
      return false;
    }
  }

  const remove = (id: string) => run(() => api.remove(id));
  const toggleFavorite = (w: Workspace) => run(() => api.setFavorite(w.id, !w.isFavorite));

  async function openExplorer(w: Workspace) {
    try {
      await api.openInExplorer(w.path);
      await api.recordOpen(w.id, "explorer");
      await load();
    } catch (e) {
      error.value = String(e);
    }
  }

  async function openTerminal(w: Workspace) {
    try {
      await api.openInTerminal(w.path);
      await api.recordOpen(w.id, "terminal");
      await load();
    } catch (e) {
      error.value = String(e);
    }
  }

  const createGroup = (payload: NewGroup) => run(() => api.createGroup(payload));
  const deleteGroup = (id: string) => run(() => api.deleteGroup(id));
  const setWorkspaceGroup = (id: string, group: string | null) =>
    run(() => api.setWorkspaceGroup(id, group));

  return {
    workspaces,
    groups,
    activeGroup,
    search,
    filtered,
    groupCounts,
    loading,
    error,
    statuses,
    gitStatuses,
    agentFilter,
    gitFilter,
    settings,
    load,
    loadSettings,
    resetSettings,
    openEditor,
    remove,
    toggleFavorite,
    openExplorer,
    openTerminal,
    createGroup,
    deleteGroup,
    setWorkspaceGroup,
    startPolling,
    stopPolling,
    launch,
    stopConfig,
    confirmAndLaunch,
    quickLaunch,
  };
}
