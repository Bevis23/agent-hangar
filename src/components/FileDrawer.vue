<script setup lang="ts">
import { computed, ref, watch } from "vue";
import Icon from "./Icon.vue";
import { api } from "../lib/api";
import { isImage, isTextViewable } from "../lib/utils";
import type { FsEntry, Workspace } from "../lib/types";

const props = defineProps<{ ws: Workspace }>();
const emit = defineEmits<{ close: []; reveal: [path: string]; view: [path: string] }>();

/** 当前浏览路径；面包屑由路径切段生成 */
const cwd = ref(props.ws.path);
const entries = ref<FsEntry[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function load() {
  loading.value = true;
  error.value = null;
  try {
    entries.value = await api.listDir(cwd.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
watch(cwd, load, { immediate: true });

/** 面包屑：把当前路径按工作空间根切段，第一段固定为空间名 */
const crumbs = computed(() => {
  const root = props.ws.path.replace(/[\\/]+$/, "");
  const cur = cwd.value.replace(/[\\/]+$/, "");
  const rest = cur.startsWith(root) ? cur.slice(root.length) : cur;
  const parts = rest.split(/[\\/]+/).filter(Boolean);
  return [{ name: props.ws.name, path: root }, ...parts.map((p, i) => ({
    name: p,
    path: root + "\\" + parts.slice(0, i + 1).join("\\"),
  }))];
});

function up() {
  const parent = cwd.value.replace(/[\\/]+$/, "").replace(/[\\/][^\\/]+$/, "");
  if (parent.length >= 3) cwd.value = parent;
}

function onOpen(e: FsEntry) {
  const p = cwd.value + "\\" + e.name;
  if (e.isDir) {
    cwd.value = p;
  } else if (isImage(e.name) || isTextViewable(e.name)) {
    // 可在应用内查看/编辑的，交给查看器弹窗
    emit("view", p);
  } else {
    api.openEntry(p).catch((err) => (error.value = String(err)));
  }
}

function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
function fmtTime(ts: number): string {
  if (!ts) return "";
  const d = new Date(ts * 1000);
  const p = (x: number) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
</script>

<template>
  <div
    class="fixed inset-0 z-40 flex justify-end bg-black/40 backdrop-blur-[2px]"
    @click.self="$emit('close')"
  >
    <aside
      class="flex h-full w-[520px] flex-col border-l border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <!-- 头部：空间名 + 操作 -->
      <div class="flex items-center gap-2 border-b border-hangar-border px-5 py-4">
        <div class="min-w-0 flex-1">
          <div class="truncate text-[15px] font-bold">{{ ws.name }}</div>
          <div class="truncate font-mono text-[11px] text-zinc-500" :title="ws.path">
            {{ ws.path }}
          </div>
        </div>
        <button
          class="btn-secondary !px-2.5 !py-1.5 text-xs"
          title="在系统资源管理器中显示当前目录"
          @click="$emit('reveal', cwd)"
        >
          资源管理器
        </button>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <!-- 面包屑 -->
      <div class="flex items-center gap-1 overflow-x-auto border-b border-hangar-border px-5 py-2.5 text-xs">
        <template v-for="(c, i) in crumbs" :key="c.path">
          <span v-if="i" class="text-zinc-600">›</span>
          <button
            class="shrink-0 whitespace-nowrap rounded px-1.5 py-0.5 transition"
            :class="i === crumbs.length - 1 ? 'text-amber-300' : 'text-zinc-400 hover:bg-zinc-800/60'"
            @click="cwd = c.path"
          >
            {{ c.name }}
          </button>
        </template>
      </div>

      <!-- 文件列表 -->
      <div class="flex-1 overflow-y-auto px-3 py-2">
        <div v-if="error" class="rounded-lg border border-red-900 bg-red-950/40 px-3 py-2 text-xs text-red-400">
          {{ error }}
        </div>
        <div v-else-if="loading" class="py-10 text-center text-xs text-zinc-500">读取中…</div>
        <div v-else-if="entries.length === 0" class="py-10 text-center text-xs text-zinc-500">
          空文件夹
        </div>
        <template v-else>
          <button
            v-if="crumbs.length > 1"
            class="dir-row"
            @click="up"
          >
            <Icon name="arrow-back" class="!w-4 !h-4 text-zinc-400" />
            <span class="flex-1 text-left text-zinc-400">..（上一级）</span>
          </button>
          <button v-for="e in entries" :key="e.name" class="dir-row" @click="onOpen(e)">
            <Icon :name="e.isDir ? 'folder' : 'file'" class="!w-5 !h-5 shrink-0 text-amber-300/90" />
            <span class="min-w-0 flex-1 text-left">
              <span class="block truncate text-[13px]" :title="e.name">{{ e.name }}</span>
              <span class="block text-[10px] text-zinc-600">{{ fmtTime(e.modified) }}</span>
            </span>
            <span class="shrink-0 font-mono text-[11px] tabular-nums text-zinc-500">
              {{ e.isDir ? "" : fmtSize(e.size) }}
            </span>
          </button>
        </template>
      </div>

      <div class="border-t border-hangar-border px-5 py-2.5 text-[11px] text-zinc-600">
        只读浏览 · 点文件夹进入，点文件用系统默认程序打开 · 不改动任何内容
      </div>
    </aside>
  </div>
</template>

<style scoped>
.dir-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 7px 10px;
  border-radius: 10px;
  transition: background 0.12s;
}
.dir-row:hover {
  background: #20232b;
}
</style>
