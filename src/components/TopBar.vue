<script setup lang="ts">
import Icon from "./Icon.vue";

defineProps<{
  search: string;
  shown: number;
  total: number;
  formationActive: boolean;
  view: "cards" | "feed";
  agentOptions: { value: string; label: string }[];
}>();
defineEmits<{
  "update:search": [v: string];
  "update:view": [v: "cards" | "feed"];
  add: [];
  import: [];
  settings: [];
  help: [];
  formation: [];
}>();
</script>

<template>
  <header
    class="sticky top-0 z-20 flex items-center gap-4 border-b border-hangar-border bg-hangar-bg/90 px-6 py-4 backdrop-blur"
  >
    <div class="flex select-none items-center gap-3">
      <div
        class="flex h-9 w-9 items-center justify-center rounded-xl bg-hangar-accent/15 text-hangar-accent"
      >
        <Icon name="send" style="width: 20px; height: 20px" />
      </div>
      <div>
        <div class="text-[15px] font-bold leading-tight tracking-wide">Hangar</div>
        <div class="text-[11px] leading-tight text-zinc-500">agent 工作空间总台</div>
      </div>
    </div>

    <div class="flex-1" />

    <div class="relative w-64">
      <Icon
        name="search"
        class="pointer-events-none absolute left-3 top-1/2 w-4 -translate-y-1/2 text-zinc-500"
      />
      <input
        :value="search"
        class="input !w-64 !pl-9 !pr-8"
        placeholder="搜索名称 / 标签 / 路径…"
        @input="$emit('update:search', ($event.target as HTMLInputElement).value)"
      />
      <button
        v-if="search"
        class="absolute right-2.5 top-1/2 -translate-y-1/2 rounded p-0.5 text-zinc-500 transition hover:text-zinc-200"
        title="清空搜索"
        @click="$emit('update:search', '')"
      >
        <Icon name="x" style="width: 13px; height: 13px" />
      </button>
    </div>

    <div class="w-14 text-center text-xs tabular-nums text-zinc-500">
      {{ shown }}/{{ total }}
    </div>

    <div class="flex overflow-hidden rounded-lg border border-hangar-border text-xs">
      <button
        class="px-2.5 py-1.5 transition"
        :class="view === 'cards' ? 'bg-amber-500/15 text-amber-300' : 'text-zinc-400 hover:text-zinc-200'"
        @click="$emit('update:view', 'cards')"
      >
        卡片
      </button>
      <button
        class="px-2.5 py-1.5 transition"
        :class="view === 'feed' ? 'bg-amber-500/15 text-amber-300' : 'text-zinc-400 hover:text-zinc-200'"
        @click="$emit('update:view', 'feed')"
      >
        <span class="inline-flex items-center gap-1"><Icon name="activity" class="!w-3.5 !h-3.5" /> 动态</span>
      </button>
    </div>

    <button
      class="icon-btn"
      title="使用指南"
      @click="$emit('help')"
    >
      <Icon name="help" />
    </button>
    <button
      class="icon-btn"
      title="设置（外观 / 终端 / 编辑器）"
      @click="$emit('settings')"
    >
      <Icon name="sliders" />
    </button>

    <button
      class="btn-secondary flex items-center gap-1.5"
      :class="{ '!border-amber-500/60 !text-amber-300': formationActive }"
      title="选择多个空间，一键同时拉起 agent"
      @click="$emit('formation')"
    >
      <span class="inline-flex items-center gap-1"><Icon name="send" class="!w-3.5 !h-3.5" /> 编队</span>
    </button>

    <button
      class="btn-secondary flex items-center gap-1.5"
      title="扫描目录批量登记"
      @click="$emit('import')"
    >
      <Icon name="folder" /> 导入
    </button>

    <button class="btn-primary flex items-center gap-1.5" @click="$emit('add')">
      <Icon name="plus" /> 登记空间
    </button>
  </header>
</template>

<style scoped>
.icon-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 8px;
  font-size: 15px;
  color: #8b8f9a;
  transition: all 0.15s;
}
.icon-btn:hover {
  background: #20232b;
  color: #e4e4e7;
}
</style>
