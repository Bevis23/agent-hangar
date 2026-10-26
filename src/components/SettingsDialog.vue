<script setup lang="ts">
import { onMounted, ref } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import Icon from "./Icon.vue";
import { BG_THEMES, COLORS } from "../lib/utils";
import { api } from "../lib/api";

const props = defineProps<{ settings: Record<string, string> }>();
const emit = defineEmits<{ close: []; reset: [] }>();

/** 开机自启开关：真实状态来自 autostart 插件，与 settings 解耦 */
const autostart = ref(false);
const autostartBusy = ref(false);

async function loadAutostart() {
  try {
    autostart.value = await api.getAutostart();
  } catch {
    autostart.value = false;
  }
}

async function toggleAutostart() {
  if (autostartBusy.value) return;
  autostartBusy.value = true;
  const target = autostart.value;
  try {
    await api.setAutostart(target);
  } catch {
    autostart.value = !target; // 失败回滚显示
  } finally {
    autostartBusy.value = false;
  }
}

onMounted(loadAutostart);

async function pickWallpaper() {
  const picked = await openFileDialog({
    multiple: false,
    title: "选择背景图片",
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp", "gif", "bmp"] }],
  });
  if (typeof picked === "string" && picked) {
    props.settings.wallpaper = picked;
  }
}

/** 从电脑选一个程序，加进「我的 agent」 */
async function pickAgentProgram() {
  const picked = await openFileDialog({
    multiple: false,
    title: "选择 agent 程序",
    filters: [{ name: "程序", extensions: ["exe", "cmd", "bat"] }],
  });
  if (typeof picked !== "string" || !picked) return;
  const fileName = (picked.split(/[\\/]/).pop() ?? "agent").replace(/\.(exe|cmd|bat)$/i, "");
  const forward = picked.replace(/\\/g, "/");
  const command = forward.includes(" ") ? `"${forward}"` : forward;
  const line = `${fileName}=${command}`;
  const cur = props.settings.custom_agents.trim();
  if (!cur.split(/\r?\n/).some((l) => l.trim() === line)) {
    props.settings.custom_agents = cur ? `${cur}\n${line}` : line;
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
        <h2 class="text-lg font-bold">设置</h2>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <div class="space-y-6 px-6 pb-6 pt-3">
        <!-- 外观 -->
        <section>
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zinc-500">外观</h3>

          <label class="lbl">强调色</label>
          <div class="mb-4 flex flex-wrap gap-2">
            <button
              v-for="c in COLORS"
              :key="c"
              class="h-7 w-7 rounded-full border-2 transition"
              :class="settings.accent === c ? 'scale-110 border-white' : 'border-transparent'"
              :style="{ background: c }"
              @click="settings.accent = c"
            />
          </div>

          <label class="lbl">背景色调</label>
          <div class="mb-4 grid grid-cols-5 gap-2">
            <button
              v-for="(t, key) in BG_THEMES"
              :key="key"
              class="rounded-xl border p-2 text-center transition"
              :class="settings.bg_theme === key ? 'border-white/70' : 'border-hangar-border hover:border-zinc-600'"
              @click="settings.bg_theme = key"
            >
              <div class="mb-1.5 h-8 w-full rounded-md" :style="{ background: t.bg }" />
              <div class="text-[11px] text-zinc-400">{{ t.label }}</div>
            </button>
          </div>

          <label class="lbl">背景图片（可选）</label>
          <div class="mb-2 flex gap-2">
            <div class="input flex flex-1 items-center truncate !text-xs" :title="settings.wallpaper">
              {{ settings.wallpaper || "未设置（使用纯色背景）" }}
            </div>
            <button class="btn-secondary shrink-0" @click="pickWallpaper">选择图片…</button>
            <button
              v-if="settings.wallpaper"
              class="btn-secondary shrink-0"
              @click="settings.wallpaper = ''"
            >
              清除
            </button>
          </div>
          <div v-if="settings.wallpaper" class="flex items-center gap-3">
            <label class="!mb-0 text-xs text-zinc-500">遮罩浓度</label>
            <input
              v-model="settings.wallpaper_dim"
              type="range"
              min="0"
              max="80"
              class="flex-1 accent-amber-400"
            />
            <span class="w-10 text-right text-xs tabular-nums text-zinc-400"
              >{{ settings.wallpaper_dim }}%</span
            >
          </div>
        </section>

        <!-- 终端与编辑器 -->
        <section>
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zinc-500">
            终端与编辑器
          </h3>

          <p class="mb-4 text-[11px] leading-relaxed text-zinc-600">
            终端自动选择（Windows Terminal 优先，PowerShell 兜底），无需配置。
          </p>

          <label class="mb-4 flex cursor-pointer items-center gap-2 text-[13px] text-zinc-300">
            <input
              type="checkbox"
              class="cbx"
              :checked="settings.auto_snapshot !== '0'"
              @change="settings.auto_snapshot = settings.auto_snapshot === '0' ? '1' : '0'"
            />
            起飞前自动快照（推荐开启——agent 改坏文件可在卡片"时间机库"一键回滚）
          </label>

          <label class="lbl">编辑器命令（卡片「编辑器」按钮，{path} 为空间目录）</label>          <div class="mb-4 rounded-xl border border-hangar-border bg-hangar-bg px-4 py-3 text-[11px] leading-relaxed text-zinc-500">
            <b class="text-zinc-300">REST API</b>（AI/脚本 HTTP 接入）：
            <code class="text-zinc-300">http://127.0.0.1:7333</code>
            <span v-if="settings.rest_token">
              ，Bearer token：<code class="text-amber-300/80">{{ settings.rest_token }}</code>（请求需带
              Authorization 头）
            </span>
          </div>

          <input
            v-model="settings.editor_cmd"
            class="input font-mono"
            placeholder="code {path}"
          />
          <div class="mt-1.5 text-[11px] leading-relaxed text-zinc-600">
            常见：VS Code 用 <code>code {path}</code>，Cursor 用 <code>cursor {path}</code>。命令需已加入
            PATH。
          </div>

          <label class="lbl mt-4">
            我的 agent（启动菜单里显示，每行一条「名字=命令」）
          </label>
          <div class="mb-2 flex gap-2">
            <textarea
              v-model="settings.custom_agents"
              rows="3"
              class="input resize-none font-mono !text-xs"
              placeholder="ZCode=D:/软件/zcode/ZCode.exe&#10;豆包=doubao"
            ></textarea>
            <button class="btn-secondary shrink-0 self-start" title="浏览电脑选择程序" @click="pickAgentProgram">
              选择程序…
            </button>
          </div>
          <div class="text-[11px] leading-relaxed text-zinc-600">
            点「选择程序…」直接浏览电脑挑 exe 即可；路径含空格会自动加引号。
          </div>
        </section>

        <!-- 系统 -->
        <section>
          <h3 class="mb-3 text-xs font-semibold uppercase tracking-wider text-zinc-500">系统</h3>

          <label class="mb-3 flex cursor-pointer items-center gap-2 text-[13px] text-zinc-300">
            <input
              type="checkbox"
              class="cbx"
              :checked="settings.close_to_tray !== '0'"
              @change="settings.close_to_tray = settings.close_to_tray === '0' ? '1' : '0'"
            />
            关闭窗口时最小化到托盘（后台常驻，Alt+H 唤起）
          </label>

          <label
            class="flex cursor-pointer items-center gap-2 text-[13px]"
            :class="autostartBusy ? 'pointer-events-none opacity-60' : 'text-zinc-300'"
          >
            <input
              type="checkbox"
              class="cbx"
              :checked="autostart"
              @change="toggleAutostart"
            />
            开机自动启动（登录时后台常驻到托盘）
          </label>
        </section>

        <div class="flex items-center justify-between border-t border-hangar-border pt-4">
          <button class="btn-secondary" @click="$emit('reset')">恢复默认</button>
          <button class="btn-primary" @click="$emit('close')">完成</button>
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
