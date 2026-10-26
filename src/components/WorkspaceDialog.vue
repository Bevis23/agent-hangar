<script setup lang="ts">
import { reactive, ref } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import Icon from "./Icon.vue";
import { api } from "../lib/api";
import type { Group, Workspace, WorkspaceInput } from "../lib/types";
import { AGENTS, COLORS, ICON_PRESETS, LAUNCH_PRESETS, basename, wsIcon } from "../lib/utils";

const props = defineProps<{
  editing: Workspace | null;
  groups: Group[];
  defaultGroup: string | null;
}>();
const emit = defineEmits<{ close: []; saved: [] }>();

const form = reactive({
  path: props.editing?.path ?? "",
  name: props.editing?.name ?? "",
  agent: props.editing?.agent ?? "claude-code",
  icon: props.editing?.icon ?? "send",
  color: props.editing?.color ?? COLORS[0],
  group: props.editing?.groupId ?? props.defaultGroup ?? "",
  tags: (props.editing?.tags ?? []).join(", "),
  notes: props.editing?.notes ?? "",
});

// 启动配置编辑（保存时整表替换，confirmSkip 保留原值）
const configs = reactive(
  (props.editing?.launchConfigs ?? []).map((c) => ({
    name: c.name,
    command: c.command,
    shell: c.shell,
    confirmSkip: c.confirmSkip,
  })),
);

function addConfig(preset?: { name: string; command: string }) {
  configs.push({
    name: preset?.name ?? "自定义命令",
    command: preset?.command ?? "",
    shell: "gitbash",
    confirmSkip: false,
  });
}
function removeConfig(i: number) {
  configs.splice(i, 1);
}

const busy = ref(false);
const error = ref<string | null>(null);

async function pickFolder() {
  const picked = await openFileDialog({
    directory: true,
    multiple: false,
    title: "选择工作空间文件夹",
  });
  if (typeof picked === "string" && picked) {
    form.path = picked;
    if (!form.name) form.name = basename(picked);
  }
}

async function submit() {
  error.value = null;
  if (!form.path) {
    error.value = "请先选择文件夹（Hangar 只登记已存在的文件夹）";
    return;
  }
  if (!form.name.trim()) {
    error.value = "请填写名称";
    return;
  }
  busy.value = true;
  try {
    const payload: WorkspaceInput = {
      name: form.name.trim(),
      path: form.path,
      agent: form.agent || null,
      notes: form.notes.trim() || null,
      icon: form.icon || null,
      color: form.color || null,
      groupId: form.group || null,
      launchConfigs: configs
        .map((c) => ({
          name: c.name.trim(),
          command: c.command.trim(),
          shell: "gitbash", // 终端由后端智能选择（有 Git 用 Git Bash，否则 PowerShell）
          confirmSkip: c.confirmSkip,
        }))
        .filter((c) => c.name && c.command),
      tags: form.tags
        .split(/[,，]/)
        .map((t) => t.trim())
        .filter(Boolean),
    };
    if (props.editing) {
      await api.update({ id: props.editing.id, ...payload });
    } else {
      await api.add(payload);
    }
    emit("saved");
  } catch (e) {
    error.value = typeof e === "string" ? e : String(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div
      class="max-h-[86vh] w-[560px] overflow-y-auto rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <div class="flex items-center justify-between px-6 pb-1 pt-5">
        <h2 class="text-lg font-bold">{{ editing ? "编辑工作空间" : "登记工作空间" }}</h2>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <div class="space-y-4 px-6 pb-6 pt-3">
        <div>
          <label class="lbl">文件夹 *</label>
          <div class="flex gap-2">
            <div
              class="input flex flex-1 items-center truncate font-mono !text-xs"
              :title="form.path"
            >
              {{ form.path || "尚未选择" }}
            </div>
            <button v-if="!editing" class="btn-secondary shrink-0" @click="pickFolder">
              选择…
            </button>
          </div>
          <div v-if="editing" class="mt-1.5 text-[11px] text-zinc-600">
            编辑时不改变文件夹本身；如需指向新位置，请重新登记。
          </div>
        </div>

        <div>
          <label class="lbl">名称 *</label>
          <input v-model="form.name" class="input" placeholder="例如：博客重构实验" />
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label class="lbl">所属 agent</label>
            <select v-model="form.agent" class="input">
              <option v-for="a in AGENTS" :key="a.value" :value="a.value">
                {{ a.label }}
              </option>
            </select>
          </div>
          <div>
            <label class="lbl">图标</label>
            <div class="input flex items-center justify-center !py-1.5">
              <Icon :name="wsIcon(form.icon)" style="width: 22px; height: 22px; color: #f5a524" />
            </div>
          </div>
        </div>

        <div class="flex flex-wrap gap-1.5">
          <button
            v-for="ic in ICON_PRESETS"
            :key="ic"
            class="emoji-pick"
            :class="{ 'emoji-pick-on': form.icon === ic }"
            :title="ic"
            @click="form.icon = ic"
          >
            <Icon :name="wsIcon(ic)" style="width: 18px; height: 18px" />
          </button>
        </div>

        <div>
          <label class="lbl">颜色</label>
          <div class="flex gap-2">
            <button
              v-for="c in COLORS"
              :key="c"
              class="h-7 w-7 rounded-full border-2 transition"
              :class="form.color === c ? 'scale-110 border-white' : 'border-transparent'"
              :style="{ background: c }"
              @click="form.color = c"
            />
          </div>
        </div>

        <div>
          <label class="lbl">标签（逗号分隔）</label>
          <input v-model="form.tags" class="input" placeholder="实验, 前端, 客户A" />
        </div>

        <div>
          <label class="lbl">分组</label>
          <select v-model="form.group" class="input">
            <option value="">不分组</option>
            <option v-for="g in groups" :key="g.id" :value="g.id">
              {{ g.icon ? g.icon + " " : "" }}{{ g.name }}
            </option>
          </select>
        </div>

        <!-- 启动配置 -->
        <div>
          <label class="lbl">启动配置（在该空间目录的终端里一键运行 agent）</label>
          <div v-if="configs.length" class="space-y-2">
            <div v-for="(c, i) in configs" :key="i" class="flex items-center gap-2">
              <input v-model="c.name" class="input lc-name" placeholder="名称" />
              <input
                v-model="c.command"
                class="input lc-cmd font-mono"
                placeholder="命令，如 claude"
              />
              <button class="lc-del" title="删除此配置" @click="removeConfig(i)">×</button>
            </div>
          </div>
          <div class="mt-2 flex flex-wrap items-center gap-1.5">
            <button
              v-for="p in LAUNCH_PRESETS"
              :key="p.name"
              class="preset-chip"
              @click="addConfig(p)"
            >
              ＋ {{ p.name }}
            </button>
            <button class="preset-chip" @click="addConfig()">＋ 自定义</button>
          </div>
        </div>

        <div>
          <label class="lbl">备注</label>
          <textarea
            v-model="form.notes"
            rows="2"
            class="input resize-none"
            placeholder="这个空间是干什么的？进展到哪了？"
          ></textarea>
        </div>

        <div
          v-if="error"
          class="rounded-lg border border-red-900 bg-red-950/50 px-3 py-2 text-sm text-red-400"
        >
          {{ error }}
        </div>

        <div class="flex justify-end gap-2 pt-1">
          <button class="btn-secondary" @click="$emit('close')">取消</button>
          <button class="btn-primary" :disabled="busy" @click="submit">
            {{ busy ? "保存中…" : "保存" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.emoji-pick {
  font-size: 16px;
  padding: 4px 6px;
  border-radius: 8px;
  background: transparent;
  border: 1px solid transparent;
  transition: all 0.15s;
}
.emoji-pick:hover {
  background: #20232b;
}
.emoji-pick-on {
  background: #20232b;
  border-color: #3a3d46;
}

/* 启动配置编辑行 */
.lc-name {
  width: 110px;
  flex-shrink: 0;
}
.lc-cmd {
  flex: 1;
  min-width: 0;
}
.lc-shell {
  width: 110px;
  flex-shrink: 0;
}
.lc-del {
  flex-shrink: 0;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  color: #565a66;
  font-size: 15px;
  transition: all 0.15s;
}
.lc-del:hover {
  background: #7f1d1d;
  color: #fecaca;
}
.preset-chip {
  padding: 3px 10px;
  border-radius: 999px;
  font-size: 12px;
  color: #a1a1aa;
  background: transparent;
  border: 1px dashed #2a2d36;
  transition: all 0.15s;
}
.preset-chip:hover {
  color: #f5c063;
  border-color: rgba(245, 165, 36, 0.5);
}
</style>
