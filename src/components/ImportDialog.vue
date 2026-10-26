<script setup lang="ts">
import { computed, ref } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import Icon from "./Icon.vue";
import { api } from "../lib/api";
import type { CandidateDir, ImportSummary } from "../lib/types";
import { AGENTS, COLORS } from "../lib/utils";

const props = defineProps<{ defaultGroupId: string | null }>();
const emit = defineEmits<{ close: []; done: [] }>();

const rootPath = ref("");
/** false = 导入全部子文件夹（默认）；true = 只看带 marker 的"像项目"文件夹 */
const onlyProjects = ref(false);
const scanning = ref(false);
const scanned = ref(false);
const candidates = ref<CandidateDir[]>([]);
const selected = ref<Set<string>>(new Set());
const defaultAgent = ref("claude-code");
const defaultTag = ref("");
const importing = ref(false);
const result = ref<ImportSummary | null>(null);
const error = ref<string | null>(null);

const selectedCount = computed(() => selected.value.size);
const allSelected = computed(
  () => candidates.value.length > 0 && selected.value.size === candidates.value.length,
);

async function pickRoot() {
  const picked = await openFileDialog({
    directory: true,
    multiple: false,
    title: "选择要扫描的根目录",
  });
  if (typeof picked === "string" && picked) {
    rootPath.value = picked;
    await scan();
  }
}

async function scan() {
  if (!rootPath.value || scanning.value) return;
  scanning.value = true;
  error.value = null;
  result.value = null;
  candidates.value = [];
  selected.value = new Set();
  scanned.value = false;
  try {
    candidates.value = await api.scanImportRoot(rootPath.value, !onlyProjects.value);
    // 默认全选：导入场景里用户通常想要"全都搬进来"
    selected.value = new Set(candidates.value.map((c) => c.path));
    scanned.value = true;
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

function toggle(path: string) {
  const next = new Set(selected.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  selected.value = next;
}

function toggleAll() {
  selected.value = allSelected.value
    ? new Set()
    : new Set(candidates.value.map((c) => c.path));
}

async function doImport() {
  if (selectedCount.value === 0 || importing.value) return;
  importing.value = true;
  error.value = null;
  try {
    const tags = defaultTag.value
      .split(/[,，]/)
      .map((t) => t.trim())
      .filter(Boolean);
    const payloads = candidates.value
      .filter((c) => selected.value.has(c.path))
      .map((c) => ({
        name: c.dirName,
        path: c.path,
        agent: defaultAgent.value || null,
        notes: null,
        icon: "folder",
        color: COLORS[Math.floor(Math.random() * COLORS.length)],
        groupId: props.defaultGroupId,
        tags,
      }));
    result.value = await api.importWorkspaces(payloads);
  } catch (e) {
    error.value = String(e);
  } finally {
    importing.value = false;
  }
}

function reset() {
  result.value = null;
  candidates.value = [];
  selected.value = new Set();
  scanned.value = false;
  rootPath.value = "";
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div
      class="max-h-[86vh] w-[680px] overflow-y-auto rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <div class="flex items-center justify-between px-6 pb-1 pt-5">
        <h2 class="text-lg font-bold">批量导入工作空间</h2>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <!-- 结果视图 -->
      <div v-if="result" class="space-y-4 px-6 pb-6 pt-3">
        <div class="rounded-xl border border-emerald-900/60 bg-emerald-950/40 px-4 py-3 text-sm">
          已登记 <b class="text-emerald-300">{{ result.added.length }}</b> 个工作空间
          <span v-if="result.failed.length" class="text-red-400">
            ，{{ result.failed.length }} 个失败</span
          >
        </div>
        <div
          v-if="result.failed.length"
          class="max-h-40 overflow-y-auto rounded-xl border border-hangar-border px-4 py-3 text-xs"
        >
          <div v-for="f in result.failed" :key="f.path" class="py-0.5">
            <span class="font-mono text-zinc-400">{{ f.path }}</span>
            <span class="text-red-400"> — {{ f.reason }}</span>
          </div>
        </div>
        <div class="flex justify-end gap-2">
          <button class="btn-secondary" @click="reset">继续导入</button>
          <button class="btn-primary" @click="$emit('done')">完成</button>
        </div>
      </div>

      <!-- 扫描视图 -->
      <div v-else class="space-y-4 px-6 pb-6 pt-3">
        <div>
          <label class="lbl">扫描根目录（深度 2 层，默认导入全部子文件夹）</label>
          <div class="flex gap-2">
            <div class="input flex flex-1 items-center truncate font-mono !text-xs" :title="rootPath">
              {{ rootPath || "尚未选择" }}
            </div>
            <button class="btn-secondary shrink-0" @click="pickRoot">选择目录…</button>
            <button
              class="btn-secondary shrink-0"
              :disabled="!rootPath || scanning"
              @click="scan"
            >
              {{ scanning ? "扫描中…" : "重新扫描" }}
            </button>
          </div>
        </div>

        <div v-if="error" class="rounded-lg border border-red-900 bg-red-950/50 px-3 py-2 text-sm text-red-400">
          {{ error }}
        </div>

        <div v-if="scanning" class="py-8 text-center text-sm text-zinc-500">
          正在扫描 {{ rootPath }} …
        </div>

        <div
          v-else-if="scanned && candidates.length === 0"
          class="rounded-xl border border-dashed border-hangar-border px-4 py-8 text-center text-sm text-zinc-500"
        >
          没有发现新的候选文件夹（识别依据：.git / package.json / Cargo.toml / CLAUDE.md 等）
        </div>

        <template v-if="!scanning && candidates.length > 0">
          <div class="flex items-center gap-2 text-xs text-zinc-500">
            <input type="checkbox" class="cbx" :checked="allSelected" @change="toggleAll" />
            <span>全选（{{ selectedCount }}/{{ candidates.length }}）</span>
            <span class="flex-1"></span>
            <label class="flex cursor-pointer items-center gap-1.5">
              <input type="checkbox" class="cbx" v-model="onlyProjects" @change="scan" />
              只看像项目的（.git / package.json 等）
            </label>
          </div>

          <div class="max-h-64 space-y-1 overflow-y-auto rounded-xl border border-hangar-border p-2">
            <label
              v-for="c in candidates"
              :key="c.path"
              class="flex cursor-pointer items-center gap-3 rounded-lg px-3 py-2 transition hover:bg-zinc-800/40"
              :class="{ 'opacity-50': !selected.has(c.path) }"
            >
              <input
                type="checkbox"
                class="cbx"
                :checked="selected.has(c.path)"
                @change="toggle(c.path)"
              />
              <div class="min-w-0 flex-1">
                <div class="text-sm font-medium">{{ c.dirName }}</div>
                <div class="truncate font-mono text-[11px] text-zinc-500" :title="c.path">
                  {{ c.path }}
                </div>
              </div>
              <div class="flex shrink-0 gap-1">
                <span
                  v-if="c.markers.length === 0"
                  class="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-500"
                >
                  普通文件夹
                </span>
                <span
                  v-for="m in c.markers.slice(0, 3)"
                  :key="m"
                  class="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-[10px] text-zinc-400"
                >
                  {{ m }}
                </span>
                <span
                  v-if="c.markers.length > 3"
                  class="rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] text-zinc-500"
                >
                  +{{ c.markers.length - 3 }}
                </span>
              </div>
            </label>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <label class="lbl">统一标记 agent</label>
              <select v-model="defaultAgent" class="input">
                <option v-for="a in AGENTS" :key="a.value" :value="a.value">
                  {{ a.label }}
                </option>
              </select>
            </div>
            <div>
              <label class="lbl">统一追加标签（可选）</label>
              <input v-model="defaultTag" class="input" placeholder="如：导入, 实验" />
            </div>
          </div>

          <div class="flex justify-end gap-2 pt-1">
            <button class="btn-secondary" @click="$emit('close')">取消</button>
            <button class="btn-primary" :disabled="selectedCount === 0 || importing" @click="doImport">
              {{ importing ? "导入中…" : `导入 ${selectedCount} 个` }}
            </button>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cbx {
  width: 15px;
  height: 15px;
  accent-color: #f5a524;
  cursor: pointer;
}
</style>
