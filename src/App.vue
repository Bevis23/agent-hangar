<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import TopBar from "./components/TopBar.vue";
import GroupBar from "./components/GroupBar.vue";
import WorkspaceCard from "./components/WorkspaceCard.vue";
import WorkspaceDialog from "./components/WorkspaceDialog.vue";
import ImportDialog from "./components/ImportDialog.vue";
import LaunchConfirmDialog from "./components/LaunchConfirmDialog.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import GuideDialog from "./components/GuideDialog.vue";
import FileDrawer from "./components/FileDrawer.vue";
import ViewerModal from "./components/ViewerModal.vue";
import SnapshotsModal from "./components/SnapshotsModal.vue";
import FormationDialog from "./components/FormationDialog.vue";
import TimelineView from "./components/TimelineView.vue";
import CommandPalette from "./components/CommandPalette.vue";
import { useWorkspaces } from "./composables/useWorkspaces";
import type { AgentInfo, LaunchConfig, Workspace } from "./lib/types";
import { AGENTS, agentLabel, basename, parseCustomAgents } from "./lib/utils";
import { api } from "./lib/api";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen } from "@tauri-apps/api/event";

// reactive() 让 composable 返回对象里嵌套的 ref 在模板中自动解包，
// 否则 store.filtered 是 Ref 对象而不是数组，GroupBar 的 counts.get 会在渲染时抛错。
const store = reactive(useWorkspaces());

const dialogOpen = ref(false);
const importOpen = ref(false);
const settingsOpen = ref(false);
const guideOpen = ref(false);
/** 编队起飞：模式开关 + 已选空间 id */
const formationMode = ref(false);
const formationSel = ref<Set<string>>(new Set());
const formationOpen = ref(false);
/** 时间机库弹窗 */
const snapshotsFor = ref<string | null>(null);
const snapshotsWs = computed(() => store.workspaces.find((w) => w.id === snapshotsFor.value) ?? null);
/** 主视图：卡片墙 / 活动时间线 */
const viewMode = ref<"cards" | "feed">("cards");
/** Alt+H 命令面板 */
const paletteOpen = ref(false);
const feedItems = ref<import("./lib/types").FeedItem[]>([]);
const feedLoading = ref(false);

async function showFeed() {
  viewMode.value = "feed";
  feedLoading.value = true;
  try {
    feedItems.value = await api.activityFeed(14);
  } catch (e) {
    store.error = String(e);
  } finally {
    feedLoading.value = false;
  }
}
/** 命令面板执行入口 */
type PaletteRun =
  | { type: "browse"; ws: Workspace }
  | { type: "launch"; ws: Workspace }
  | { type: "action"; id: "add" | "import" | "formation" | "feed" | "settings" };

function runPalette(a: PaletteRun) {
  switch (a.type) {
    case "browse":
      browsing.value = a.ws;
      break;
    case "launch": {
      const cfg = a.ws.launchConfigs[0];
      if (cfg) onLaunch(a.ws.path, cfg);
      else openEdit(a.ws.id);
      break;
    }
    case "action":
      if (a.id === "add") openAdd();
      else if (a.id === "import") importOpen.value = true;
      else if (a.id === "settings") settingsOpen.value = true;
      else if (a.id === "formation") {
        formationMode.value = true;
        formationSel.value = new Set();
      } else if (a.id === "feed") showFeed();
      break;
  }
}

function pickFromFeed(name: string) {
  viewMode.value = "cards";
  store.search = name;
}
const editingId = ref<string | null>(null);
/** 待确认的启动（confirmSkip=false 的配置点击后先弹确认） */
const pendingLaunch = ref<{ lc: LaunchConfig; dir: string } | null>(null);
/** 「启动…」菜单的快捷启动（同样先确认） */
const pendingQuick = ref<{ ws: Workspace; name: string; command: string } | null>(null);
/** 应用内文件浏览抽屉 */
const browsing = ref<Workspace | null>(null);
/** 应用内文件查看/编辑弹窗 */
const viewing = ref<{ path: string; name: string } | null>(null);
/** 本机已安装的 agent（启动菜单用） */
const installedAgents = ref<AgentInfo[]>([]);
/** 设置里手动添加的"我的 agent" */
const customAgents = computed(() => parseCustomAgents(store.settings.custom_agents));
/** agent 筛选选项：内置主流名单 ∪ 卡片上实际出现的 agent ∪ 我的 agent（去重排序） */
const agentOptions = computed(() => {
  const seen = new Set<string>();
  const out: { value: string; label: string }[] = [];
  const push = (v: string) => {
    if (!v || seen.has(v)) return;
    seen.add(v);
    out.push({ value: v, label: agentLabel(v) });
  };
  AGENTS.forEach((a) => push(a.value));
  store.workspaces.forEach((w) => push(w.agent ?? ""));
  customAgents.value.forEach((a) => push(a.name));
  // 未知名排在内置之后、manual 之前（agentLabel 对未知值原样显示）
  return out.filter((a) => a.value !== "manual").concat(out.filter((a) => a.value === "manual"));
});

function openAdd() {
  editingId.value = null;
  dialogOpen.value = true;
}
function openEdit(id: string) {
  editingId.value = id;
  dialogOpen.value = true;
}

function onLaunch(dir: string, lc: LaunchConfig) {
  if (lc.confirmSkip) {
    store.launch(lc.id);
  } else {
    pendingLaunch.value = { lc, dir };
  }
}

/** 把一条 agent 追加进「我的 agent」（去重） */
function appendCustomAgent(name: string, command: string) {
  const cur = store.settings.custom_agents.trim();
  const line = `${name}=${command}`;
  if (!cur.split(/\r?\n/).some((l) => l.trim() === line)) {
    store.settings.custom_agents = cur ? `${cur}\n${line}` : line;
  }
}

/** 「启动…」菜单选择：save=true 时同时保存到「我的 agent」。终端由后端智能选择。 */
function onQuick(ws: Workspace, name: string, command: string, save: boolean) {
  if (save) appendCustomAgent(name, command);
  pendingQuick.value = { ws, name, command };
}

function toggleFormation(ws: Workspace) {
  const next = new Set(formationSel.value);
  if (next.has(ws.id)) next.delete(ws.id);
  else next.add(ws.id);
  formationSel.value = next;
}
function exitFormation() {
  formationMode.value = false;
  formationSel.value = new Set();
}
const selectedWorkspaces = computed(() =>
  store.workspaces.filter((w) => formationSel.value.has(w.id)),
);

/** 编队起飞完成：清空选择并刷新（agent 已由编队逐个拉起） */
function onFormationDone() {
  formationOpen.value = false;
  exitFormation();
  store.load();
  store.startPolling();
}

// 当前分组过滤生效时，新登记/导入的空间默认归入该分组
const dialogDefaultGroup = computed(() =>
  store.activeGroup && store.activeGroup !== "none" ? store.activeGroup : null,
);

const groupName = computed(() => {
  const map = new Map(
    store.groups.map((g) => [g.id, g.icon ? `${g.icon} ${g.name}` : g.name] as const),
  );
  return (id: string | null) => (id ? (map.get(id) ?? null) : null);
});

/** 视图切换代理：切到"动态"时拉取时间线 */
const viewModeProxy = computed({
  get: () => viewMode.value,
  set: (v: "cards" | "feed") => (v === "feed" ? showFeed() : (viewMode.value = "cards")),
});

onMounted(() => {
  store.load();
  store.loadSettings();
  store.startPolling();
  api
    .listInstalledAgents()
    .then((a) => (installedAgents.value = a))
    .catch(() => void 0);
  // 拖拽登记：把文件夹拖进窗口直接收进机库（文件夹原地不动，只登记）
  getCurrentWebview()
    .onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      const paths = event.payload.paths.filter((p) => p && !p.includes(".hangar"));
      for (const p of paths) {
        api
          .add({
            name: basename(p),
            path: p,
            agent: null,
            notes: null,
            icon: "folder",
            color: null,
            groupId: dialogDefaultGroup.value,
            launchConfigs: [],
            tags: ["拖拽"],
          })
          .catch(() => void 0); // 已登记等失败静默，界面刷新会反映真实状态
      }
      window.setTimeout(() => store.load(), 400);
    })
    .then((un) => (unlistenDrag = un))
    .catch(() => void 0);
  // 托盘点收藏空间（无配置）→ 定位浏览
  listen("browse-workspace", (e) => {
    const id = (e as unknown as { payload: string }).payload;
    const ws = store.workspaces.find((w) => w.id === id);
    if (ws) browsing.value = ws;
  }).then((un) => (unlistenBrowse = un))
    .catch(() => void 0);
  // 全局快捷键 Alt+H（后端发来）→ 命令面板开关
  listen("palette-toggle", () => {
    paletteOpen.value = !paletteOpen.value;
    if (paletteOpen.value) store.load();
  }).then((un) => (unlistenPal = un))
    .catch(() => void 0);
  // 窗口重新聚焦时刷新：CLI/MCP 改了数据、或后台变更，回来看到的总是最新的
  window.addEventListener("focus", refreshOnFocus);
});
onBeforeUnmount(() => {
  store.stopPolling();
  window.removeEventListener("focus", refreshOnFocus);
  if (unlistenDrag) unlistenDrag();
  if (unlistenPal) unlistenPal();
  if (unlistenBrowse) unlistenBrowse();
});

let lastFocusRefresh = 0;
function refreshOnFocus() {
  const now = Date.now();
  if (now - lastFocusRefresh < 800) return; // 防抖
  lastFocusRefresh = now;
  store.load();
  store.loadSettings();
  if (viewMode.value === "feed") showFeed();
}
let unlistenDrag: (() => void) | undefined;
let unlistenPal: (() => void) | undefined;
let unlistenBrowse: (() => void) | undefined;
</script>

<template>
  <div class="min-h-screen">
    <!-- 背景壁纸层（设置里选择了图片时显示，带可调遮罩保证内容可读）。
         本地文件必须走 Tauri asset 协议（convertFileSrc），直接写盘符路径会被网页源拦截。 -->
    <div
      v-if="store.settings.wallpaper"
      class="fixed inset-0 -z-10 bg-cover bg-center"
      :style="{
        backgroundImage: `linear-gradient(rgba(11,12,15,${Number(store.settings.wallpaper_dim) / 100}), rgba(11,12,15,${Number(store.settings.wallpaper_dim) / 100})), url('${convertFileSrc(store.settings.wallpaper)}')`,
      }"
    />

    <TopBar
      v-model:search="store.search"
      :shown="store.filtered.length"
      :total="store.workspaces.length"
      :formation-active="formationMode"
      :agent-options="agentOptions"
      v-model:view="viewModeProxy"
      @add="openAdd"
      @import="importOpen = true"
      @settings="settingsOpen = true"
      @help="guideOpen = true"
      @formation="
        formationMode = !formationMode;
        formationSel = new Set();
      "
    />

    <GroupBar
      v-if="viewMode === 'cards'"
      v-model:active="store.activeGroup"
      :groups="store.groups"
      :counts="store.groupCounts"
      :total="store.workspaces.length"
      @create="store.createGroup"
      @remove="store.deleteGroup"
    />

    <!-- 筛选行：agent / git 状态 -->
    <div v-if="store.workspaces.length && viewMode === 'cards'" class="flex items-center justify-end gap-2 px-6 pt-3">
      <select v-model="store.agentFilter" class="input flt">
        <option value="">全部 agent</option>
        <option v-for="a in agentOptions" :key="a.value" :value="a.value">{{ a.label }}</option>
      </select>
      <select v-model="store.gitFilter" class="input flt">
        <option value="">git 不限</option>
        <option value="dirty">有未提交修改</option>
        <option value="clean">工作区干净</option>
        <option value="nonrepo">非 git 目录</option>
      </select>
    </div>

    <div
      v-if="store.error"
      class="mx-6 mt-4 rounded-lg border border-red-900 bg-red-950/40 px-4 py-3 text-sm text-red-400"
    >
      {{ store.error }}
    </div>

    <main v-if="viewMode === 'feed'" class="px-0 py-0">
      <div v-if="feedLoading" class="py-20 text-center text-sm text-zinc-500">装载动态…</div>
      <TimelineView v-else :items="feedItems" @pick="pickFromFeed" />
    </main>

    <main v-else class="px-6 py-5">
      <div
        v-if="store.loading && store.workspaces.length === 0"
        class="py-20 text-center text-sm text-zinc-500"
      >
        装载中…
      </div>

      <div v-else-if="store.workspaces.length === 0" class="empty">
        <Icon name="send" style="width: 56px; height: 56px" class="mx-auto text-amber-300" filled />
        <h2 class="mt-4 text-xl font-bold">机库还是空的</h2>
        <p class="max-w-md text-center text-sm leading-relaxed text-zinc-500">
          把散落各处的 agent 工作空间登记进来。文件夹原地不动——<br />
          Hangar 只做登记、整理与一键到达。
        </p>
        <div class="mt-4 flex gap-2">
          <button class="btn-secondary" @click="importOpen = true">扫描目录导入</button>
          <button class="btn-primary" @click="openAdd">＋ 登记工作空间</button>
        </div>
      </div>

      <div
        v-else-if="store.filtered.length === 0"
        class="py-16 text-center text-sm text-zinc-500"
      >
        没有匹配的工作空间{{ store.search ? `「${store.search}」` : "" }}
      </div>

      <div
        v-else
        class="grid gap-4"
        style="grid-template-columns: repeat(auto-fill, minmax(300px, 1fr))"
      >
        <WorkspaceCard
          v-for="ws in store.filtered"
          :key="ws.id"
          :ws="ws"
          :group="groupName(ws.groupId)"
          :statuses="store.statuses"
          :git="store.gitStatuses[ws.path] ?? null"
          :installed-agents="installedAgents"
          :custom-agents="customAgents"
          :formation-mode="formationMode"
          :selected="formationSel.has(ws.id)"
          @open-files="browsing = ws"
          @open-editor="store.openEditor(ws)"
          @edit="openEdit(ws.id)"
          @delete="store.remove(ws.id)"
          @toggle-favorite="store.toggleFavorite(ws)"
          @toggle-formation="toggleFormation(ws)"
          @snapshots="snapshotsFor = ws.id"
          @launch="(lc) => onLaunch(ws.path, lc)"
          @stop="(lc) => store.stopConfig(lc.id)"
          @quick="(name, command, save) => onQuick(ws, name, command, save)"
          @add-agent="(name, command) => appendCustomAgent(name, command)"
        />
      </div>

      <!-- 编队模式浮动操作栏 -->
      <div
        v-if="formationMode"
        class="fixed bottom-8 left-1/2 z-30 flex -translate-x-1/2 items-center gap-3 rounded-2xl border border-amber-500/50 bg-hangar-panel/95 px-5 py-3 shadow-2xl backdrop-blur"
      >
        <span class="text-sm">
          已选 <b class="text-amber-300">{{ formationSel.size }}</b> 个空间
        </span>
        <button
          class="btn-primary"
          :disabled="formationSel.size === 0"
          @click="formationOpen = true"
        >
          <span class="inline-flex items-center gap-1.5"><Icon name="send" class="!w-4 !h-4" filled /> 编队起飞</span>
        </button>
        <button class="btn-secondary" @click="exitFormation">取消</button>
      </div>
    </main>

    <LaunchConfirmDialog
      v-if="pendingLaunch"
      :config="pendingLaunch.lc"
      :dir="pendingLaunch.dir"
      @close="pendingLaunch = null"
      @confirm="
        (remember) => {
          const p = pendingLaunch;
          pendingLaunch = null;
          if (p) store.confirmAndLaunch(p.lc.id, remember);
        }
      "
    />

    <LaunchConfirmDialog
      v-else-if="pendingQuick"
      :config="{
        id: '',
        workspaceId: pendingQuick.ws.id,
        name: pendingQuick.name,
        command: pendingQuick.command,
        shell: 'gitbash',
        confirmSkip: false,
      }"
      :dir="pendingQuick.ws.path"
      @close="pendingQuick = null"
      @confirm="
        (remember) => {
          const p = pendingQuick;
          pendingQuick = null;
          if (p) store.quickLaunch(p.ws.id, p.name, p.command, remember);
        }
      "
    />

    <WorkspaceDialog
      v-if="dialogOpen"
      :editing="store.workspaces.find((w) => w.id === editingId) ?? null"
      :groups="store.groups"
      :default-group="dialogDefaultGroup"
      @close="dialogOpen = false"
      @saved="dialogOpen = false; store.load()"
    />

    <ImportDialog
      v-if="importOpen"
      :default-group-id="dialogDefaultGroup"
      @close="importOpen = false"
      @done="importOpen = false; store.load()"
    />

    <SettingsDialog
      v-if="settingsOpen"
      :settings="store.settings"
      @close="settingsOpen = false"
      @reset="store.resetSettings()"
    />

    <FileDrawer
      v-if="browsing"
      :ws="browsing"
      @close="browsing = null"
      @view="(path) => (viewing = { path, name: path.split(/[\\/]/).pop() ?? path })"
      @reveal="
        (path) => {
          api.openInExplorer(path).catch((e) => (store.error = String(e)));
          if (browsing) api.recordOpen(browsing.id, 'explorer');
        }
      "
    />

    <ViewerModal
      v-if="viewing"
      :path="viewing.path"
      :name="viewing.name"
      @close="viewing = null"
    />

    <CommandPalette
      v-if="paletteOpen"
      :workspaces="store.workspaces"
      @close="paletteOpen = false"
      @run="runPalette"
    />

    <GuideDialog v-if="guideOpen" @close="guideOpen = false" />

    <SnapshotsModal v-if="snapshotsWs" :ws="snapshotsWs" @close="snapshotsFor = null" />

    <FormationDialog
      v-if="formationOpen"
      :workspaces="selectedWorkspaces"
      :installed-agents="installedAgents"
      :custom-agents="customAgents"
      @close="formationOpen = false"
      @done="onFormationDone"
    />
  </div>
</template>


