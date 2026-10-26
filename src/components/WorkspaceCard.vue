<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import Icon from "./Icon.vue";
import type { AgentInfo, GitStatus, LaunchConfig, Workspace } from "../lib/types";
import { agentLabel, relTime, wsIcon } from "../lib/utils";

const props = defineProps<{
  ws: Workspace;
  group?: string | null;
  statuses: Record<string, string>;
  git?: GitStatus | null;
  installedAgents: AgentInfo[];
  customAgents: { name: string; command: string }[];
  formationMode?: boolean;
  selected?: boolean;
}>();
const emit = defineEmits<{
  openFiles: [];
  openEditor: [];
  edit: [];
  delete: [];
  toggleFavorite: [];
  toggleFormation: [];
  snapshots: [];
  launch: [lc: LaunchConfig];
  stop: [lc: LaunchConfig];
  quick: [name: string, command: string, save: boolean];
  addAgent: [name: string, command: string];
}>();

function isRunning(lc: LaunchConfig) {
  return props.statuses[lc.id] === "running";
}

// 「启动…」菜单：检测到的 agent + 我的 agent + 自定义命令
const menuOpen = ref(false);
const customCmd = ref("");

function pick(name: string, command: string, save: boolean) {
  menuOpen.value = false;
  if (command.trim()) emit("quick", name, command.trim(), save);
}
function pickCustom(save: boolean) {
  const cmd = customCmd.value.trim();
  if (!cmd) return;
  const name = cmd.split(/\s+/)[0].replace(/\.exe$/i, "").split("/").pop() ?? cmd;
  pick(name, cmd, save);
  customCmd.value = "";
}

/** 从电脑里选一个程序加进「我的 agent」（名字取文件名，路径转正斜杠） */
async function addAgentFromDisk() {
  const picked = await openFileDialog({
    multiple: false,
    title: "选择 agent 程序",
    filters: [{ name: "程序", extensions: ["exe", "cmd", "bat"] }],
  });
  if (typeof picked !== "string" || !picked) return;
  const fileName = (picked.split(/[\\/]/).pop() ?? "agent").replace(/\.(exe|cmd|bat)$/i, "");
  const forward = picked.replace(/\\/g, "/");
  const command = forward.includes(" ") ? `"${forward}"` : forward;
  menuOpen.value = false;
  emit("addAgent", fileName, command);
}

// 两段式停止确认：运行中点一次变红"确认停止?"，再点才真正终止进程树。
const stopArmed = ref<string | null>(null);
let stopTimer: number | undefined;

function onStopClick(lc: LaunchConfig) {
  if (stopArmed.value === lc.id) {
    disarmStop();
    emit("stop", lc);
    return;
  }
  stopArmed.value = lc.id;
  stopTimer = window.setTimeout(disarmStop, 2600);
}
function disarmStop() {
  stopArmed.value = null;
  if (stopTimer) window.clearTimeout(stopTimer);
}

// 两段式删除确认：第一次点击武装，2.6 秒内再点一次才真正删除。
const deleteArmed = ref(false);
let disarmTimer: number | undefined;

function onDeleteClick() {
  if (deleteArmed.value) {
    disarm();
    emit("delete");
    return;
  }
  deleteArmed.value = true;
  disarmTimer = window.setTimeout(disarm, 2600);
}
function disarm() {
  deleteArmed.value = false;
  if (disarmTimer) window.clearTimeout(disarmTimer);
}
onBeforeUnmount(() => {
  disarm();
  disarmStop();
});
</script>

<template>
  <div
    class="group relative rounded-2xl border p-4 transition-all hover:-translate-y-0.5"
    :class="
      selected
        ? 'border-amber-500/70 bg-amber-950/20'
        : 'border-hangar-border bg-hangar-panel hover:border-zinc-600'
    "
    :style="{ zIndex: menuOpen ? 50 : undefined }"
  >
    <!-- 编队模式：左上角选择圈 -->
    <button
      v-if="formationMode"
      class="absolute left-3 top-3 z-10 flex h-6 w-6 items-center justify-center rounded-full border-2 text-[11px] font-bold transition"
      :class="
        selected
          ? 'border-amber-400 bg-amber-400 text-black'
          : 'border-zinc-500 text-transparent hover:border-amber-400'
      "
      title="选入编队"
      @click="$emit('toggleFormation')"
    >
      ✓
    </button>

    <button
      class="absolute right-3 top-3 rounded-md p-1 transition"
      :class="
        ws.isFavorite
          ? 'text-hangar-accent'
          : 'text-zinc-600 opacity-0 hover:text-zinc-300 group-hover:opacity-100'
      "
      :title="ws.isFavorite ? '取消收藏' : '收藏'"
      @click="$emit('toggleFavorite')"
    >
      <Icon name="star" :filled="ws.isFavorite" />
    </button>

    <div class="flex items-start gap-3 pr-7" :class="formationMode ? 'pl-8' : ''">
      <div
        class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl text-xl"
        :style="{ background: (ws.color ?? '#3f3f46') + '26', color: ws.color ?? '#a1a1aa' }"
      >
        <Icon :name="wsIcon(ws.icon)" style="width: 22px; height: 22px" />
      </div>
      <div class="min-w-0 flex-1">
        <div class="truncate text-[15px] font-semibold" :title="ws.name">{{ ws.name }}</div>
        <div class="truncate font-mono text-xs text-zinc-500" :title="ws.path">
          {{ ws.path }}
        </div>
      </div>
    </div>

    <div class="mt-3 flex min-h-[22px] flex-wrap items-center gap-1.5">
      <span
        class="rounded-full border border-hangar-border bg-zinc-800/70 px-2 py-0.5 text-[11px] text-zinc-400"
      >
        {{ agentLabel(ws.agent) }}
      </span>
      <span
        v-if="group"
        class="rounded-full border border-sky-900/50 bg-sky-950/40 px-2 py-0.5 text-[11px] text-sky-300/80"
      >
        {{ group }}
      </span>
      <span
        v-if="git?.isRepo"
        class="git-chip"
        :class="git.dirty > 0 ? 'git-dirty' : 'git-clean'"
        :title="git.dirty > 0 ? `${git.dirty} 个文件未提交` : '工作区干净'"
      >
        {{ git.branch ?? "(无提交)" }}<template v-if="git.dirty > 0"> ~{{ git.dirty }}</template>
      </span>
      <span
        v-for="t in ws.tags"
        :key="t"
        class="rounded-full bg-hangar-accent/10 px-2 py-0.5 text-[11px] text-amber-300/90"
      >
        #{{ t }}
      </span>
    </div>

    <div v-if="ws.notes" class="mt-2 line-clamp-2 text-xs leading-relaxed text-zinc-500">
      {{ ws.notes }}
    </div>

    <div class="mt-2 text-[11px] text-zinc-600">最后打开 {{ relTime(ws.lastOpenedAt) }}</div>

    <!-- agent 启动按钮行：已保存的配置 + 「启动…」快捷菜单 -->
    <div class="relative mt-2 flex flex-wrap gap-1.5">
      <button
        v-for="lc in ws.launchConfigs"
        :key="lc.id"
        class="lbtn"
        :class="{ 'lbtn-run': isRunning(lc), 'lbtn-armed': stopArmed === lc.id }"
        :title="
          isRunning(lc)
            ? '运行中 —— 点击停止（终止终端与其中进程）'
            : `运行：${lc.command}`
        "
        @click="isRunning(lc) ? onStopClick(lc) : $emit('launch', lc)"
      >
        <span v-if="isRunning(lc)" class="dot"></span>
        <Icon v-else name="play" class="!w-2.5 !h-2.5 text-hangar-accent" filled />
        {{ stopArmed === lc.id ? "确认停止？" : lc.name }}
      </button>

      <button class="lbtn lbtn-menu" title="选择 agent 在此空间启动" @click.stop="menuOpen = !menuOpen">
        <Icon name="play" class="!w-2.5 !h-2.5 text-hangar-accent" filled /> 启动…
      </button>

      <template v-if="menuOpen">
        <div class="fixed inset-0 z-30" @click="menuOpen = false"></div>
        <div class="launch-menu">
          <div class="menu-title">检测到的 agent</div>
          <button
            v-for="a in installedAgents"
            :key="a.id"
            class="menu-item"
            @click="pick(a.name, a.command)"
          >
            <Icon name="play" class="!w-2 !h-2 text-hangar-accent" filled />{{ a.name }}<span class="mi-cmd">{{ a.command }}</span>
          </button>
          <div v-if="!installedAgents.length" class="menu-empty">
            未检测到已安装的 agent——可在「设置 → 我的 agent」手动添加
          </div>
          <template v-if="customAgents.length">
            <div class="menu-title">我的 agent</div>
            <button
              v-for="a in customAgents"
              :key="a.command"
              class="menu-item"
              :title="a.command"
              @click="pick(a.name, a.command, false)"
            >
              <Icon name="play" class="!w-2 !h-2 text-hangar-accent" filled />{{ a.name }}<span class="mi-cmd">{{ a.command }}</span>
            </button>
          </template>
          <button class="menu-add" title="从电脑里选一个程序加入启动菜单" @click="addAgentFromDisk">
            ＋ 从电脑添加 agent…
          </button>
          <div class="menu-title">自定义命令</div>
          <div class="menu-row">
            <input
              v-model="customCmd"
              class="menu-input"
              placeholder="如 claude --continue（回车启动）"
              @keyup.enter="pickCustom(false)"
            />
            <button
              class="menu-save"
              title="保存到「我的 agent」，以后直接出现在菜单里"
              @click="pickCustom(true)"
            >
              存为常用
            </button>
          </div>
        </div>
      </template>
    </div>

    <div
      class="mt-3 flex items-center gap-1 border-t border-hangar-border pt-3 opacity-0 transition group-hover:opacity-100"
    >
      <button class="act" title="在 Hangar 内浏览该文件夹" @click="$emit('openFiles')">
        <Icon name="folder" /> 浏览
      </button>
      <button class="act" title="时间机库：快照与回滚" @click="$emit('snapshots')">
        <Icon name="history" />
      </button>
      <button class="act" title="用编辑器打开（右上角设置里配置命令）" @click="$emit('openEditor')">
        <Icon name="code" />
      </button>
      <div class="flex-1" />
      <button class="act" title="编辑" @click="$emit('edit')">
        <Icon name="pencil" />
      </button>
      <button
        class="act"
        :class="{ 'danger-armed': deleteArmed }"
        :title="deleteArmed ? '再点一次确认移除登记（不会删除文件夹）' : '移除登记'"
        @click="onDeleteClick"
      >
        <Icon name="trash" />{{ deleteArmed ? "确认?" : "" }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.act {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 8px;
  font-size: 12px;
  color: #a1a1aa;
  transition: all 0.15s;
}
.act:hover {
  background: #20232b;
  color: #e4e4e7;
}
.act.danger-armed,
.act.danger-armed:hover {
  background: #7f1d1d;
  color: #fecaca;
}

/* agent 启动按钮 */
.lbtn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 8px;
  font-size: 12px;
  color: #d4d4d8;
  background: #1d2027;
  border: 1px solid #2a2d36;
  transition: all 0.15s;
}
.lbtn:hover {
  border-color: rgba(245, 165, 36, 0.5);
  color: #f5c063;
}
.lbtn .play {
  font-size: 9px;
  color: #f5a524;
}
.lbtn-run {
  background: rgba(52, 211, 153, 0.1);
  border-color: rgba(52, 211, 153, 0.4);
  color: #6ee7b7;
}
.lbtn-run .dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: #34d399;
  animation: pulse 1.6s ease-in-out infinite;
}
.lbtn-armed,
.lbtn-armed:hover {
  background: #7f1d1d !important;
  border-color: #b91c1c !important;
  color: #fecaca !important;
}
@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.35;
  }
}

/* git 状态 chip */
.git-chip {
  border-radius: 999px;
  padding: 0 8px;
  font-size: 11px;
  font-family: Consolas, "Cascadia Mono", monospace;
  line-height: 20px;
  max-width: 160px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.git-clean {
  background: rgba(52, 211, 153, 0.08);
  color: #6ee7b7;
  border: 1px solid rgba(52, 211, 153, 0.25);
}
.git-dirty {
  background: rgba(245, 165, 36, 0.1);
  color: #fbbf24;
  border: 1px solid rgba(245, 165, 36, 0.35);
}

/* 启动…菜单 */
.lbtn-menu {
  border-style: dashed;
}
.launch-menu {
  position: absolute;
  top: calc(100% + 6px);
  left: 0;
  z-index: 40;
  min-width: 240px;
  padding: 6px;
  border-radius: 12px;
  background: #191c22;
  border: 1px solid #2a2d36;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.5);
}
.menu-title {
  padding: 6px 8px 3px;
  font-size: 10px;
  color: #6b7080;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.menu-item {
  display: flex;
  align-items: center;
  gap: 7px;
  width: 100%;
  padding: 7px 8px;
  border-radius: 8px;
  font-size: 13px;
  color: #d4d4d8;
  text-align: left;
  transition: background 0.12s;
}
.menu-item:hover {
  background: #232732;
}
.menu-item .play {
  font-size: 9px;
  color: #f5a524;
}
.mi-cmd {
  margin-left: auto;
  font-family: Consolas, monospace;
  font-size: 11px;
  color: #6b7080;
}
.menu-empty {
  padding: 6px 8px;
  font-size: 11px;
  color: #6b7080;
}
.menu-input {
  width: 100%;
  margin-top: 2px;
  padding: 7px 8px;
  border-radius: 8px;
  border: 1px solid #2a2d36;
  background: #0f1115;
  color: #e7e9ee;
  font-size: 12px;
  font-family: Consolas, monospace;
  outline: none;
}
.menu-input:focus {
  border-color: rgba(245, 165, 36, 0.5);
}
.menu-row {
  display: flex;
  gap: 6px;
  margin-top: 2px;
}
.menu-row .menu-input {
  flex: 1;
  margin-top: 0;
}
.menu-save {
  flex-shrink: 0;
  padding: 6px 9px;
  border-radius: 8px;
  font-size: 11px;
  color: #f5c063;
  background: rgba(245, 165, 36, 0.08);
  border: 1px solid rgba(245, 165, 36, 0.35);
  transition: all 0.15s;
}
.menu-save:hover {
  background: rgba(245, 165, 36, 0.18);
}
.menu-add {
  display: block;
  width: 100%;
  padding: 7px 8px;
  border-radius: 8px;
  font-size: 12px;
  color: #8b8f9a;
  border: 1px dashed #2a2d36;
  text-align: center;
  transition: all 0.15s;
  margin: 4px 0 2px;
}
.menu-add:hover {
  color: #f5c063;
  border-color: rgba(245, 165, 36, 0.5);
}
</style>
