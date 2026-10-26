<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { marked } from "marked";
import DOMPurify from "dompurify";
import { convertFileSrc } from "@tauri-apps/api/core";
import Icon from "./Icon.vue";
import { api } from "../lib/api";
import { isImage, isTextViewable } from "../lib/utils";

const props = defineProps<{ path: string; name: string }>();
const emit = defineEmits<{ close: []; saved: [] }>();

const isImg = computed(() => isImage(props.name));
const isText = computed(() => isTextViewable(props.name));

const content = ref("");
const editing = ref(false);
const dirty = ref(false);
const truncated = ref(false);
const loading = ref(true);
const error = ref<string | null>(null);
const savedFlash = ref(false);

const rendered = computed(() =>
  DOMPurify.sanitize(marked.parse(content.value, { async: false }) as string),
);

async function load() {
  loading.value = true;
  error.value = null;
  editing.value = false;
  dirty.value = false;
  try {
    const tf = await api.readTextFile(props.path);
    content.value = tf.content;
    truncated.value = tf.truncated;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
watch(() => props.path, load, { immediate: true });

async function save() {
  error.value = null;
  try {
    await api.writeTextFile(props.path, content.value);
    dirty.value = false;
    savedFlash.value = true;
    window.setTimeout(() => (savedFlash.value = false), 1500);
    emit("saved");
  } catch (e) {
    error.value = String(e);
  }
}
function onKey(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === "s") {
    e.preventDefault();
    if (isText.value && dirty.value) save();
  }
  if (e.key === "Escape" && !dirty.value) emit("close");
}
window.addEventListener("keydown", onKey);
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));

function confirmClose() {
  if (!dirty.value || window.confirm("有未保存的修改，确定关闭？")) emit("close");
}
const imgSrc = computed(() => convertFileSrc(props.path));
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="confirmClose"
  >
    <div
      class="flex max-h-[88vh] w-[820px] max-w-full flex-col overflow-hidden rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <!-- 头部 -->
      <div class="flex items-center gap-2 border-b border-hangar-border px-5 py-3">
        <Icon :name="isImg ? 'camera' : 'file'" class="!w-5 !h-5 text-amber-300" />
        <div class="min-w-0 flex-1">
          <div class="truncate text-sm font-bold" :title="path">{{ name }}</div>
          <div class="truncate font-mono text-[10px] text-zinc-600">{{ path }}</div>
        </div>
        <span
          v-if="savedFlash"
          class="rounded-full bg-emerald-900/50 px-2.5 py-0.5 text-[11px] text-emerald-300"
          >已保存 ✓</span
        >
        <template v-if="isText && !truncated">
          <button
            v-if="name.toLowerCase().endsWith('.md')"
            class="btn-secondary !px-2.5 !py-1.5 text-xs"
            @click="editing = !editing"
          >
            {{ editing ? "预览" : "编辑" }}
          </button>
          <button
            class="btn-primary !px-2.5 !py-1.5 text-xs"
            :disabled="!dirty"
            @click="save"
          >
            保存
          </button>
        </template>
        <button
          class="btn-secondary !px-2.5 !py-1.5 text-xs"
          title="用系统默认程序打开"
          @click="api.openEntry(path).catch((e) => (error = String(e)))"
        >
          系统打开
        </button>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="confirmClose">
          <Icon name="x" />
        </button>
      </div>

      <!-- 内容区 -->
      <div class="min-h-0 flex-1 overflow-y-auto">
        <div v-if="loading" class="py-16 text-center text-sm text-zinc-500">读取中…</div>
        <div v-else-if="error" class="m-5 rounded-lg border border-red-900 bg-red-950/40 px-4 py-3 text-sm text-red-400">
          {{ error }}
        </div>

        <!-- 图片 -->
        <div v-else-if="isImg" class="flex justify-center p-6">
          <img :src="imgSrc" class="max-h-[60vh] rounded-lg" :alt="name" />
        </div>

        <!-- markdown：预览/编辑双模式 -->
        <template v-else-if="isText && name.toLowerCase().endsWith('.md')">
          <div v-if="!editing" class="md-body px-8 py-6" v-html="rendered"></div>
          <textarea
            v-else
            v-model="content"
            @input="dirty = true"
            class="h-full min-h-[55vh] w-full resize-none bg-hangar-bg p-6 font-mono text-[13px] leading-relaxed text-zinc-200 outline-none"
            spellcheck="false"
          ></textarea>
        </template>

        <!-- 其他文本：直接编辑 -->
        <textarea
          v-else-if="isText"
          v-model="content"
          @input="dirty = true"
          class="h-full min-h-[55vh] w-full resize-none bg-hangar-bg p-6 font-mono text-[13px] leading-relaxed text-zinc-200 outline-none"
          spellcheck="false"
        ></textarea>

        <!-- 不可预览：提示用系统打开 -->
        <div v-else class="py-16 text-center text-sm text-zinc-500">
          该文件类型无法在应用内预览
          <div class="mt-3">
            <button class="btn-secondary" @click="api.openEntry(path)">用系统默认程序打开</button>
          </div>
        </div>
      </div>

      <div
        v-if="truncated"
        class="border-t border-amber-900/50 bg-amber-950/30 px-5 py-1.5 text-[11px] text-amber-300"
      >
        文件超过 2MB，仅显示前 2MB 内容（保存会覆盖截断后的版本，请谨慎）
      </div>
      <div v-else-if="isText" class="border-t border-hangar-border px-5 py-1.5 text-[11px] text-zinc-600">
        Ctrl+S 保存 · 编辑即存本机 · {{ dirty ? "有未保存修改" : "无未保存修改" }}
      </div>
    </div>
  </div>
</template>

<style scoped>
/* markdown 预览的基础排版（自己写，免引样式库） */
.md-body {
  font-size: 14px;
  line-height: 1.75;
  color: #d4d4d8;
  word-break: break-word;
}
.md-body :deep(h1) { font-size: 1.6em; font-weight: 700; margin: 0.8em 0 0.5em; color: #fafafa; }
.md-body :deep(h2) { font-size: 1.35em; font-weight: 700; margin: 0.8em 0 0.4em; color: #fafafa; }
.md-body :deep(h3) { font-size: 1.15em; font-weight: 600; margin: 0.7em 0 0.4em; }
.md-body :deep(p) { margin: 0.6em 0; }
.md-body :deep(ul), .md-body :deep(ol) { padding-left: 1.6em; margin: 0.5em 0; }
.md-body :deep(li) { margin: 0.25em 0; }
.md-body :deep(code) {
  background: #1d2027;
  border-radius: 4px;
  padding: 0.1em 0.4em;
  font-family: Consolas, "Cascadia Mono", monospace;
  font-size: 0.9em;
  color: #f5c063;
}
.md-body :deep(pre) {
  background: #101216;
  border: 1px solid #242730;
  border-radius: 10px;
  padding: 12px 14px;
  overflow-x: auto;
  margin: 0.7em 0;
}
.md-body :deep(pre code) { background: none; padding: 0; color: #d4d4d8; }
.md-body :deep(blockquote) {
  border-left: 3px solid #f5a52455;
  margin: 0.7em 0;
  padding: 0.2em 0 0.2em 1em;
  color: #a1a1aa;
}
.md-body :deep(a) { color: #38bdf8; text-decoration: underline; }
.md-body :deep(table) { border-collapse: collapse; margin: 0.7em 0; }
.md-body :deep(th), .md-body :deep(td) { border: 1px solid #2a2d36; padding: 5px 10px; }
.md-body :deep(img) { max-width: 100%; border-radius: 8px; }
.md-body :deep(hr) { border: none; border-top: 1px solid #242730; margin: 1em 0; }
</style>
