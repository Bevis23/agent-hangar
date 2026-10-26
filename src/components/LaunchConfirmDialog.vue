<script setup lang="ts">
import { ref } from "vue";
import Icon from "./Icon.vue";
import type { LaunchConfig } from "../lib/types";

defineProps<{ config: LaunchConfig; dir: string }>();
const emit = defineEmits<{ close: []; confirm: [remember: boolean] }>();

const remember = ref(false);
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div class="w-[520px] rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl">
      <div class="flex items-center justify-between px-6 pb-1 pt-5">
        <h2 class="text-lg font-bold">确认启动</h2>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <div class="space-y-4 px-6 pb-6 pt-3">
        <p class="text-sm leading-relaxed text-zinc-400">
          即将启动 <b class="text-zinc-200">「{{ config.name }}」</b>，将执行以下命令：
        </p>

        <div class="rounded-xl border border-hangar-border bg-hangar-bg px-4 py-3">
          <div class="break-all font-mono text-sm text-amber-300">$ {{ config.command }}</div>
          <div class="mt-2 break-all font-mono text-[11px] text-zinc-500">{{ dir }}</div>
        </div>

        <p class="text-[11px] text-zinc-600">终端自动选择：有 Git 用 Git Bash，否则 PowerShell。</p>

        <label class="flex cursor-pointer items-center gap-2 text-xs text-zinc-400">
          <input v-model="remember" type="checkbox" class="cbx" />
          记住选择，此配置以后启动不再询问
        </label>

        <div class="flex justify-end gap-2">
          <button class="btn-secondary" @click="$emit('close')">取消</button>
          <button class="btn-primary" @click="$emit('confirm', remember)">启动</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cbx {
  width: 14px;
  height: 14px;
  accent-color: #f5a524;
  cursor: pointer;
}
</style>
