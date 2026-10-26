<script setup lang="ts">
import { computed, ref } from "vue";
import Icon from "./Icon.vue";
import { api } from "../lib/api";
import { wsIcon } from "../lib/utils";
import type { AgentInfo, Workspace } from "../lib/types";

const props = defineProps<{
  workspaces: Workspace[];
  installedAgents: AgentInfo[];
  customAgents: { name: string; command: string }[];
}>();
const emit = defineEmits<{ close: []; done: [ok: number, fail: number] }>();

const phase = ref<"pick" | "running" | "result">("pick");
const ok = ref(0);
const fail = ref(0);

/** 起飞计划：每个空间优先用自己的第一条启动配置；没配置的才用下方"补选机型" */
const plan = computed(() =>
  props.workspaces.map((ws) => ({
    ws,
    own: ws.launchConfigs[0] ?? null,
  })),
);
const needsFallback = computed(() => plan.value.some((p) => !p.own));
const fallback = ref<{ name: string; command: string } | null>(null);
const canFly = computed(() => !needsFallback.value || !!fallback.value);
/** 桌面单实例应用（.exe 路径）提示：编队只会唤醒同一个窗口 */
const exeHint = computed(
  () =>
    fallback.value?.command.toLowerCase().includes(".exe") ??
    props.workspaces.some((w) =>
      (w.launchConfigs[0]?.command ?? "").toLowerCase().includes(".exe"),
    ),
);

const agentOptions = computed(() => [
  ...props.installedAgents.map((a) => ({ name: a.name, command: a.command })),
  ...props.customAgents.map((a) => ({ name: a.name, command: a.command })),
]);

/** 编队起飞：有配置的用自己的配置起飞，没配置的用补选机型 */
async function fly() {
  phase.value = "running";
  ok.value = 0;
  fail.value = 0;
  for (const p of plan.value) {
    try {
      if (p.own) {
        await api.launch(p.own.id);
      } else if (fallback.value) {
        await api.quickLaunch({
          workspaceId: p.ws.id,
          name: fallback.value.name,
          command: fallback.value.command,
          shell: "gitbash",
          confirmSkip: true,
        });
      } else {
        fail.value++;
        continue;
      }
      ok.value++;
    } catch {
      fail.value++;
    }
  }
  phase.value = "result";
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="phase !== 'running' && $emit('close')"
  >
    <div
      class="max-h-[80vh] w-[600px] overflow-y-auto rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <div class="flex items-center justify-between px-6 pb-1 pt-5">
        <div>
          <h2 class="flex items-center gap-2 text-lg font-bold"><Icon name="send" class="!w-5 !h-5 text-amber-300" /> 编队起飞</h2>
          <div class="text-[11px] text-zinc-500">
            各飞各的：有启动配置的空间用自己的配置，没配置的临时选机型 · 起飞前自动快照
          </div>
        </div>
        <button
          v-if="phase !== 'running'"
          class="text-zinc-500 transition hover:text-zinc-200"
          @click="$emit('close')"
        >
          <Icon name="x" />
        </button>
      </div>

      <div class="space-y-4 px-6 pb-6 pt-3">
        <!-- 起飞计划 -->
        <template v-if="phase === 'pick'">
          <div class="max-h-48 space-y-1 overflow-y-auto rounded-xl border border-hangar-border p-2">
            <div
              v-for="p in plan"
              :key="p.ws.id"
              class="flex items-center gap-2 px-2 py-1.5 text-sm"
            >
              <Icon :name="wsIcon(p.ws.icon)" class="!w-4 !h-4 text-zinc-300" />
              <span class="truncate">{{ p.ws.name }}</span>
              <span class="ml-auto shrink-0 text-[11px]" :class="p.own ? 'text-emerald-300' : 'text-zinc-500'">
                {{ p.own ? `${p.own.name}（自己的配置）` : "未配置" }}
              </span>
            </div>
          </div>

          <!-- 补选机型：仅当有空间缺配置 -->
          <div v-if="needsFallback">
            <label class="lbl">为「未配置」的空间选择机型</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                v-for="a in agentOptions"
                :key="a.command"
                class="menu-item border"
                :class="
                  fallback?.command === a.command
                    ? 'border-amber-500/60 bg-amber-950/20'
                    : 'border-hangar-border'
                "
                @click="fallback = a"
              >
                <Icon name="play" class="!w-2.5 !h-2.5 text-hangar-accent" filled />{{ a.name
                }}<span class="mi-cmd">{{ a.command }}</span>
              </button>
            </div>
            <div v-if="!agentOptions.length" class="mt-2 text-xs text-zinc-500">
              未检测到 agent——先在卡片「启动…」菜单里「从电脑添加 agent…」
            </div>
            <div
              v-if="exeHint"
              class="mt-2 rounded-lg border border-amber-900/50 bg-amber-950/30 px-3 py-2 text-[11px] leading-relaxed text-amber-300"
            >
              提示：豆包 / WorkBuddy / ZCode 等桌面应用是单实例的——编队不会为每个空间开新窗口，
              只会唤醒同一个应用。适合编队的是 claude / codex 这类命令行 agent。
            </div>
          </div>

          <div class="flex items-center justify-between pt-1">
            <span class="text-xs text-zinc-500">
              {{ needsFallback && !fallback ? "先选机型才能起飞" : "准备就绪" }}
            </span>
            <button class="btn-primary" :disabled="!canFly" @click="fly">
              起飞（{{ workspaces.length }} 个空间）
            </button>
          </div>
        </template>

        <!-- 起飞中 -->
        <div v-else-if="phase === 'running'" class="py-12 text-center">
          <Icon name="send" style="width: 40px; height: 40px" class="mx-auto text-amber-300" />
          <div class="mt-3 animate-pulse text-sm text-zinc-400">
            编队起飞中… {{ ok + fail }}/{{ workspaces.length }}
          </div>
        </div>

        <!-- 结果 -->
        <div v-else class="space-y-3 py-4 text-center">
          <Icon name="send" style="width: 40px; height: 40px" class="mx-auto text-emerald-300" filled />
          <div class="text-lg font-bold">
            <span class="text-emerald-300">{{ ok }} 架已起飞</span>
            <span v-if="fail" class="text-red-400">，{{ fail }} 架失败</span>
          </div>
          <div class="text-xs text-zinc-500">每架起飞前已自动快照，可在卡片时间机库里回滚</div>
          <button class="btn-primary mt-2" @click="$emit('done', ok, fail)">完成</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.menu-item {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 9px 12px;
  border-radius: 10px;
  font-size: 13px;
  color: #d4d4d8;
  text-align: left;
  transition: all 0.15s;
}
.menu-item:hover {
  background: #20232b;
}
.menu-item .play {
  font-size: 10px;
  color: #f5a524;
}
.mi-cmd {
  margin-left: auto;
  font-family: Consolas, monospace;
  font-size: 10px;
  color: #6b7080;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
