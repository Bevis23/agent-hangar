<script setup lang="ts">
import { computed } from "vue";
import Icon from "./Icon.vue";
import type { FeedItem } from "../lib/types";

const props = defineProps<{ items: FeedItem[] }>();
const emit = defineEmits<{ pick: [name: string] }>();

const KIND_ICON: Record<string, string> = {
  launch: "send",
  commit: "code",
  work: "pencil",
  snapshot: "package",
  open: "folder",
};

/** agent 贡献画像：本周（数据窗口内）每只 agent 干的活计数 */
const agentStats = computed(() => {
  const counts = new Map<string, number>();
  for (const it of props.items) {
    if (it.kind !== "work" && it.kind !== "launch" && it.kind !== "commit") continue;
    const a = it.agent?.trim();
    if (!a || a === "manual") continue;
    counts.set(a, (counts.get(a) ?? 0) + 1);
  }
  return [...counts.entries()].sort((x, y) => y[1] - x[1]);
});

/** 按天分组：今天 / 昨天 / M月D日 */
const days = computed(() => {
  const groups = new Map<string, FeedItem[]>();
  const now = new Date();
  const todayKey = now.toISOString().slice(0, 10);
  const yestKey = new Date(now.getTime() - 86400000).toISOString().slice(0, 10);
  for (const it of props.items) {
    const key = new Date(it.ts * 1000).toISOString().slice(0, 10);
    const label = key === todayKey ? "今天" : key === yestKey ? "昨天" : key.slice(5).replace("-", "月") + "日";
    if (!groups.has(label)) groups.set(label, []);
    groups.get(label)!.push(it);
  }
  return [...groups.entries()];
});

function hhmm(ts: number): string {
  const d = new Date(ts * 1000);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
</script>

<template>
  <div class="mx-auto max-w-3xl px-6 py-5">
    <!-- agent 贡献画像 -->
    <div v-if="agentStats.length" class="mb-5 flex flex-wrap items-center gap-2">
      <span class="text-xs text-zinc-500">本周期贡献：</span>
      <span
        v-for="[agent, n] in agentStats"
        :key="agent"
        class="rounded-full border border-hangar-border bg-hangar-panel px-3 py-1 text-xs"
      >
        <span class="inline-flex items-center gap-1"><Icon name="bot" class="!w-3.5 !h-3.5" /> {{ agent }} <b class="text-amber-300">×{{ n }}</b></span>
      </span>
    </div>

    <div v-if="items.length === 0" class="py-16 text-center text-sm text-zinc-500">
      最近还没有活动——起飞一个 agent、或让它在工作里用 MCP 记一笔，这里就会亮起来
    </div>

    <div v-for="[day, list] in days" :key="day" class="mb-6">
      <div class="mb-2 flex items-center gap-3">
        <span class="text-xs font-semibold text-zinc-400">{{ day }}</span>
        <span class="h-px flex-1 bg-hangar-border"></span>
        <span class="text-[11px] text-zinc-600">{{ list.length }} 条</span>
      </div>
      <div class="space-y-1">
        <div
          v-for="(it, i) in list"
          :key="`${it.ts}-${i}`"
          class="flex items-start gap-3 rounded-xl border border-hangar-border bg-hangar-panel px-4 py-2.5"
        >
          <Icon :name="KIND_ICON[it.kind] ?? 'code'" class="pt-0.5 !w-4 !h-4 shrink-0 text-amber-300" />
          <div class="min-w-0 flex-1">
            <div class="truncate text-[13px] text-zinc-200">{{ it.text }}</div>
            <div class="mt-0.5 flex items-center gap-2 text-[11px] text-zinc-500">
              <button
                class="rounded bg-zinc-800/70 px-1.5 py-0.5 transition hover:text-amber-300"
                :title="`筛选「${it.wsName}」`"
                @click="$emit('pick', it.wsName)"
              >
                {{ it.wsName }}
              </button>
              <span v-if="it.agent && it.agent !== 'manual'" class="inline-flex items-center gap-1 text-zinc-600"><Icon name="bot" class="!w-3 !h-3" /> {{ it.agent }}</span>
            </div>
          </div>
          <span class="shrink-0 pt-1 font-mono text-[11px] tabular-nums text-zinc-600">
            {{ hhmm(it.ts) }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>
