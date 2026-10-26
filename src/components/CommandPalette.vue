<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import Icon from "./Icon.vue";
import type { Workspace } from "../lib/types";

/** 面板条目：空间浏览 / 空间起飞 / 全局动作 */
export interface PaletteItem {
  id: string;
  icon: string;
  title: string;
  sub: string;
  run: () => void;
}

const props = defineProps<{ workspaces: Workspace[] }>();
const emit = defineEmits<{ close: []; run: [item: PaletteItem] }>();

const query = ref("");
const input = ref<HTMLInputElement | null>(null);
const idx = ref(0);

/** 全局动作（固定集合） */
const ACTIONS: PaletteItem[] = [
  { id: "act-add", icon: "plus", title: "登记工作空间", sub: "手动添加一个文件夹", run: () => emit("run", { type: "action", id: "add" }) },
  { id: "act-import", icon: "layers", title: "导入工作空间", sub: "扫描目录批量登记", run: () => emit("run", { type: "action", id: "import" }) },
  { id: "act-formation", icon: "send", title: "编队起飞", sub: "多选空间一键各自起飞", run: () => emit("run", { type: "action", id: "formation" }) },
  { id: "act-feed", icon: "activity", title: "查看动态", sub: "时间线与 agent 贡献画像", run: () => emit("run", { type: "action", id: "feed" }) },
  { id: "act-settings", icon: "settings", title: "打开设置", sub: "外观 / 编辑器 / 我的 agent", run: () => emit("run", { type: "action", id: "settings" }) },
];

const items = computed<PaletteItem[]>(() => {
  const out: PaletteItem[] = [];
  for (const ws of props.workspaces) {
    out.push({
      id: `browse-${ws.id}`,
      icon: "folder",
      title: `浏览 ${ws.name}`,
      sub: ws.path,
      run: () => emit("run", { type: "browse", ws }),
    });
    const cfg = ws.launchConfigs[0];
    if (cfg) {
      out.push({
        id: `launch-${ws.id}`,
        icon: "send",
        title: `起飞 ${ws.name}`,
        sub: `${cfg.command} · 起飞前自动快照`,
        run: () => emit("run", { type: "launch", ws }),
      });
    }
  }
  out.push(...ACTIONS);
  return out;
});

/** 模糊过滤：按输入顺序匹配 title+sub+path */
const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) {
    // 无输入：先动作后空间，各取前若干
    return [...ACTIONS, ...items.value.filter((i) => i.id.startsWith("browse-")).slice(0, 8)];
  }
  return items.value.filter((i) =>
    `${i.title} ${i.sub}`.toLowerCase().includes(q),
  );
});

watch([query, filtered], () => (idx.value = 0));
watch(input, (v) => v?.focus(), { immediate: true });

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") {
    e.preventDefault();
    emit("close");
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    idx.value = Math.min(idx.value + 1, filtered.value.length - 1);
    scrollActive();
    return;
  }
  if (e.key === "ArrowUp") {
    e.preventDefault();
    idx.value = Math.max(idx.value - 1, 0);
    scrollActive();
    return;
  }
  if (e.key === "Enter") {
    e.preventDefault();
    const it = filtered.value[idx.value];
    if (it) {
      emit("close");
      it.run();
    }
  }
}
window.addEventListener("keydown", onKey);
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));

function scrollActive() {
  requestAnimationFrame(() => {
    document.getElementById(`pal-${idx.value}`)?.scrollIntoView({ block: "nearest" });
  });
}
</script>

<template>
  <div
    class="fixed inset-0 z-[60] flex items-start justify-center bg-black/50 p-6 pt-[12vh] backdrop-blur-[2px]"
    @click.self="$emit('close')"
  >
    <div
      class="w-full max-w-[560px] overflow-hidden rounded-2xl border border-zinc-600/70 bg-hangar-panel shadow-2xl"
    >
      <div class="flex items-center gap-3 border-b border-hangar-border px-4 py-3">
        <Icon name="search" class="!h-4 !w-4 shrink-0 text-zinc-500" />
        <input
          ref="input"
          v-model="query"
          class="w-full bg-transparent text-[14px] text-zinc-100 outline-none placeholder:text-zinc-600"
          placeholder="搜索空间或命令…（↑↓ 选择，回车执行）"
          spellcheck="false"
        />
        <span class="shrink-0 rounded border border-hangar-border px-1.5 py-0.5 text-[10px] text-zinc-500">Esc</span>
      </div>

      <div class="max-h-[52vh] overflow-y-auto p-2">
        <div v-if="filtered.length === 0" class="py-8 text-center text-[13px] text-zinc-500">
          没有匹配的结果
        </div>
        <button
          v-for="(it, i) in filtered"
          :id="`pal-${i}`"
          :key="it.id"
          type="button"
          class="flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-left transition"
          :class="i === idx ? 'bg-amber-500/12 ring-1 ring-amber-500/40' : 'hover:bg-zinc-800/50'"
          @mouseenter="idx = i"
          @click="
            () => {
              $emit('close');
              it.run();
            }
          "
        >
          <Icon :name="it.icon" class="!h-4 !w-4 shrink-0" :class="i === idx ? 'text-amber-300' : 'text-zinc-500'" />
          <span class="min-w-0 flex-1">
            <span class="block truncate text-[13.5px]" :class="i === idx ? 'text-zinc-100' : 'text-zinc-300'">{{ it.title }}</span>
            <span class="block truncate font-mono text-[10.5px] text-zinc-600">{{ it.sub }}</span>
          </span>
          <Icon v-if="i === idx" name="play" class="!h-3 !w-3 shrink-0 text-amber-300" filled />
        </button>
      </div>

      <div class="flex items-center gap-3 border-t border-hangar-border px-4 py-2 text-[10.5px] text-zinc-600">
        <span>↑↓ 选择</span><span>回车 执行</span><span>Esc 关闭</span>
        <span class="flex-1"></span>
        <span>{{ filtered.length }} 项</span>
      </div>
    </div>
  </div>
</template>
