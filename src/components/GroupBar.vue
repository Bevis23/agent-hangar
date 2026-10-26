<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import type { Group } from "../lib/types";
import { GROUP_ALL, GROUP_NONE } from "../composables/useWorkspaces";

defineProps<{
  groups: Group[];
  active: string;
  counts: Map<string, number>;
  total: number;
}>();
const emit = defineEmits<{
  "update:active": [v: string];
  create: [payload: { name: string; icon: string | null }];
  remove: [id: string];
}>();

// 新建分组：内联小表单
const creating = ref(false);
const newName = ref("");
const newIcon = ref("");
const nameInput = ref<HTMLInputElement | null>(null);

function openCreate() {
  creating.value = true;
  requestAnimationFrame(() => nameInput.value?.focus());
}
function submitCreate() {
  if (!newName.value.trim()) return;
  emit("create", { name: newName.value.trim(), icon: newIcon.value.trim() || null });
  newName.value = "";
  newIcon.value = "";
  creating.value = false;
}

// 两段式删除分组：确认后删除，组内工作空间变为"未分组"
const armId = ref<string | null>(null);
let timer: number | undefined;

function onRemove(g: Group) {
  if (armId.value === g.id) {
    if (timer) window.clearTimeout(timer);
    armId.value = null;
    emit("remove", g.id);
    return;
  }
  armId.value = g.id;
  timer = window.setTimeout(() => (armId.value = null), 2600);
}
onBeforeUnmount(() => timer && window.clearTimeout(timer));
</script>

<template>
  <div class="flex flex-wrap items-center gap-2 px-6 pt-4">
    <button
      class="chip"
      :class="{ 'chip-on': active === GROUP_ALL }"
      @click="$emit('update:active', GROUP_ALL)"
    >
      全部 <span class="cnt">{{ total }}</span>
    </button>
    <button
      class="chip"
      :class="{ 'chip-on': active === GROUP_NONE }"
      @click="$emit('update:active', GROUP_NONE)"
    >
      未分组 <span class="cnt">{{ counts.get(GROUP_NONE) ?? 0 }}</span>
    </button>
    <button
      v-for="g in groups"
      :key="g.id"
      class="chip"
      :class="{ 'chip-on': active === g.id }"
      @click="$emit('update:active', g.id)"
    >
      <span v-if="g.icon">{{ g.icon }}</span>
      {{ g.name }}
      <span class="cnt">{{ counts.get(g.id) ?? 0 }}</span>
      <span
        class="del"
        :class="{ 'del-armed': armId === g.id }"
        title="删除分组（组内空间变为未分组，不会被删除）"
        @click.stop="onRemove(g)"
        >×</span
      >
    </button>

    <div v-if="creating" class="flex items-center gap-1.5">
      <input
        ref="nameInput"
        v-model="newName"
        class="input inline-input"
        placeholder="分组名"
        @keyup.enter="submitCreate"
      />
      <input
        v-model="newIcon"
        class="input inline-input icon-input"
        placeholder="图标"
        maxlength="2"
        title="可选 emoji 图标"
      />
      <button class="btn-primary btn-mini" @click="submitCreate">创建</button>
      <button class="btn-secondary btn-mini" @click="creating = false">取消</button>
    </div>
    <button v-else class="chip chip-add" title="新建分组" @click="openCreate">＋ 分组</button>
  </div>
</template>

<style scoped>
.chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  color: #a1a1aa;
  background: #15171c;
  border: 1px solid #242730;
  transition: all 0.15s;
}
.chip:hover {
  color: #e4e4e7;
  border-color: #3f434e;
}
.chip-on {
  background: rgba(245, 165, 36, 0.12);
  border-color: rgba(245, 165, 36, 0.45);
  color: #f5c063;
}
.chip-add {
  border-style: dashed;
}
.cnt {
  font-size: 11px;
  color: #6b7080;
}
.chip-on .cnt {
  color: rgba(245, 165, 36, 0.6);
}
.del {
  margin-left: 2px;
  padding: 0 3px;
  border-radius: 6px;
  color: #565a66;
  font-size: 13px;
  line-height: 1;
  opacity: 0;
  transition: all 0.15s;
}
.chip:hover .del {
  opacity: 1;
}
.del:hover {
  color: #f87171;
}
.del-armed,
.del-armed:hover {
  opacity: 1;
  background: #7f1d1d;
  color: #fecaca;
}
.inline-input {
  width: 120px;
  padding: 5px 10px;
  font-size: 12px;
}
.icon-input {
  width: 44px;
  text-align: center;
}
.btn-mini {
  padding: 5px 10px;
  font-size: 12px;
}
</style>
