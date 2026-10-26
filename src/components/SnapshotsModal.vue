<script setup lang="ts">
import { ref } from "vue";
import Icon from "./Icon.vue";
import { api } from "../lib/api";
import type { SnapInfo, Workspace } from "../lib/types";

const props = defineProps<{ ws: Workspace }>();
const emit = defineEmits<{ close: [] }>();

const snaps = ref<SnapInfo[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
/** 恢复结果显示 */
const restored = ref<number | null>(null);
const delArm = ref<string | null>(null);

async function load() {
  loading.value = true;
  try {
    snaps.value = await api.listSnapshots(props.ws.id);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
load();

async function restore(s: SnapInfo) {
  if (!window.confirm(`把「${props.ws.name}」回滚到这份快照？\n当前文件将被快照内容覆盖！`)) return;
  error.value = null;
  try {
    restored.value = await api.restoreSnapshot(props.ws.id, s.file, props.ws.path);
    await load();
  } catch (e) {
    error.value = String(e);
  }
}

async function remove(s: SnapInfo) {
  if (delArm.value !== s.file) {
    delArm.value = s.file;
    window.setTimeout(() => (delArm.value = null), 2600);
    return;
  }
  delArm.value = null;
  try {
    await api.deleteSnapshot(props.ws.id, s.file);
    await load();
  } catch (e) {
    error.value = String(e);
  }
}

function fmtTime(ts: number): string {
  const d = new Date(ts * 1000);
  const p = (x: number) => String(x).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div
      class="max-h-[80vh] w-[560px] overflow-y-auto rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <div class="flex items-center justify-between px-6 pb-1 pt-5">
        <div>
          <h2 class="flex items-center gap-2 text-lg font-bold"><Icon name="history" class="!w-5 !h-5 text-amber-300" /> 时间机库</h2>
          <div class="text-[11px] text-zinc-500">{{ ws.name }} · 每次起飞前自动快照，保留最近 5 份</div>
        </div>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <div class="space-y-2 px-6 pb-6 pt-3">
        <div
          v-if="restored !== null"
          class="rounded-xl border border-emerald-900/60 bg-emerald-950/40 px-4 py-2.5 text-sm text-emerald-300"
        >
          已回滚，恢复了 {{ restored }} 个文件
        </div>
        <div
          v-if="error"
          class="rounded-lg border border-red-900 bg-red-950/40 px-3 py-2 text-xs text-red-400"
        >
          {{ error }}
        </div>

        <div v-if="loading" class="py-10 text-center text-sm text-zinc-500">读取中…</div>
        <div v-else-if="snaps.length === 0" class="py-10 text-center text-sm text-zinc-500">
          还没有快照——下一次 agent 起飞时会自动生成
        </div>

        <div
          v-for="s in snaps"
          :key="s.file"
          class="flex items-center gap-3 rounded-xl border border-hangar-border px-4 py-3"
        >
          <Icon name="package" class="!w-5 !h-5 text-amber-300" />
          <div class="min-w-0 flex-1">
            <div class="text-sm font-medium">{{ fmtTime(s.createdAt) }}</div>
            <div class="text-[11px] text-zinc-500">快照体积 {{ fmtSize(s.size) }}（不含依赖目录）</div>
          </div>
          <button class="btn-primary !px-3 !py-1.5 text-xs" @click="restore(s)">回滚到此</button>
          <button
            class="rounded-lg px-2 py-1.5 text-xs transition"
            :class="
              delArm === s.file
                ? 'bg-red-900 text-red-200'
                : 'text-zinc-500 hover:bg-zinc-800 hover:text-zinc-300'
            "
            @click="remove(s)"
          >
            {{ delArm === s.file ? "确认删除?" : "删除" }}
          </button>
        </div>

        <div class="pt-1 text-[11px] leading-relaxed text-zinc-600">
          快照保存在本机 %APPDATA%\hangar\snapshots\，不含 node_modules 等依赖目录；回滚会用快照内容覆盖当前文件。
        </div>
      </div>
    </div>
  </div>
</template>
