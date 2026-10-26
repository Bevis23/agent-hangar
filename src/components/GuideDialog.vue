<script setup lang="ts">
import { ref } from "vue";
import Icon from "./Icon.vue";

defineEmits<{ close: [] }>();

const tab = ref<"quick" | "card" | "ai">("quick");

const TABS = [
  { id: "quick", label: "60 秒上手" },
  { id: "card", label: "读懂卡片" },
  { id: "ai", label: "AI 接入" },
] as const;
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-6 backdrop-blur-sm"
    @click.self="$emit('close')"
  >
    <div
      class="flex max-h-[86vh] w-[680px] flex-col overflow-hidden rounded-2xl border border-hangar-border bg-hangar-panel shadow-2xl"
    >
      <!-- 头部 -->
      <div class="flex items-center justify-between px-6 pb-2 pt-5">
        <h2 class="flex items-center gap-2 text-lg font-bold">
          <Icon name="help" class="!h-5 !w-5 text-hangar-accent" /> 使用指南
        </h2>
        <button class="text-zinc-500 transition hover:text-zinc-200" @click="$emit('close')">
          <Icon name="x" />
        </button>
      </div>

      <!-- Tab 切换 -->
      <div class="flex gap-1 border-b border-hangar-border px-6">
        <button
          v-for="t in TABS"
          :key="t.id"
          class="relative px-3 pb-2.5 pt-1 text-[13px] transition"
          :class="tab === t.id ? 'font-semibold text-amber-300' : 'text-zinc-500 hover:text-zinc-300'"
          @click="tab = t.id"
        >
          {{ t.label }}
          <span
            v-if="tab === t.id"
            class="absolute inset-x-2 bottom-0 h-0.5 rounded-full bg-hangar-accent"
          ></span>
        </button>
      </div>

      <!-- 内容 -->
      <div class="min-h-0 flex-1 space-y-5 overflow-y-auto px-6 pb-6 pt-4 text-sm leading-relaxed text-zinc-300">
        <!-- ================= 60 秒上手 ================= -->
        <template v-if="tab === 'quick'">
          <section>
            <h3 class="mb-3 font-semibold text-zinc-100">四步开始</h3>
            <ol class="space-y-2.5">
              <li class="flex gap-3">
                <span class="step-num">1</span>
                <div class="flex-1 text-[13px] text-zinc-400">
                  点右上角「<b class="text-zinc-200">导入</b>」，选你的项目根目录（比如
                  <code>D:\workbuddy_space</code>）——当前层的全部子文件夹都会列出来，默认全选，点导入
                </div>
              </li>
              <li class="flex gap-3">
                <span class="step-num">2</span>
                <div class="flex-1 text-[13px] text-zinc-400">
                  随便找张卡片，点「<b class="text-zinc-200">启动…</b>」——菜单里是你电脑上检测到的 AI
                  （Claude Code / Codex…），点一个 → 确认 → AI 已经在该目录里跑起来了，窗口保持打开
                </div>
              </li>
              <li class="flex gap-3">
                <span class="step-num">3</span>
                <div class="flex-1 text-[13px] text-zinc-400">
                  第二条分组条「＋ 分组」建几个组（比如 在干活 / 烂尾 / 学习），编辑卡片归组；
                  顶栏搜索、右侧下拉按 AI / git 状态筛选
                </div>
              </li>
              <li class="flex gap-3">
                <span class="step-num">4</span>
                <div class="flex-1 text-[13px] text-zinc-400">
                  切到「<b class="text-zinc-200">动态</b>」视图——起飞、git 提交、AI
                  的工作汇报都在时间线上，顶部还有每只 AI 的贡献计数
                </div>
              </li>
            </ol>
          </section>

          <section class="rounded-xl border border-hangar-border p-4">
            <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">
              常用动作速查
            </h3>
            <div class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-[12.5px]">
              <span class="text-zinc-200">拖文件夹进窗口</span><span class="text-zinc-500">直接登记进机库</span>
              <span class="text-zinc-200">编队（顶栏）</span><span class="text-zinc-500">多选空间一键各自起飞</span>
              <span class="text-zinc-200">卡片 · 时钟图标</span><span class="text-zinc-500">时间机库：起飞前自动快照，一键回滚</span>
              <span class="text-zinc-200">卡片 · 浏览</span><span class="text-zinc-500">应用内翻文件；点 md 可预览/编辑</span>
              <span class="text-zinc-200">⚙ 设置</span><span class="text-zinc-500">换主题/壁纸、配编辑器、加"我的 agent"</span>
            </div>
          </section>
        </template>

        <!-- ================= 读懂卡片 ================= -->
        <template v-else-if="tab === 'card'">
          <!-- 模拟卡片示意 -->
          <div class="rounded-xl border border-hangar-border bg-hangar-bg p-4">
            <div class="flex items-center gap-3">
              <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-amber-400/15 text-amber-300">
                <Icon name="book" style="width: 20px; height: 20px" />
              </div>
              <div class="min-w-0 flex-1">
                <div class="text-[14px] font-semibold">python学习</div>
                <div class="truncate font-mono text-[10px] text-zinc-500">D:\workbuddy_space\python学习</div>
              </div>
              <Icon name="star" class="!h-4 !w-4 text-zinc-600" />
            </div>
            <div class="mt-2.5 flex flex-wrap items-center gap-1.5">
              <span class="chip-demo">WorkBuddy</span>
              <span class="chip-demo amber">#python</span>
              <span class="chip-demo amber">#学习</span>
              <span class="chip-demo git-dirty">main ~3</span>
            </div>
            <div class="mt-2 text-[11px] text-zinc-500">最后打开 3 小时前 · [10-25 19:27] 完成第三章练习</div>
            <div class="mt-2.5 flex gap-1.5">
              <span class="launch-demo"><Icon name="play" class="!h-2.5 !w-2.5" filled /> WorkBuddy</span>
              <span class="launch-demo dashed"><Icon name="play" class="!h-2.5 !w-2.5" filled /> 启动…</span>
            </div>
          </div>

          <section class="space-y-2">
            <div class="flex gap-2.5">
              <Icon name="code" class="mt-0.5 !h-4 !w-4 shrink-0 text-emerald-300" />
              <p class="text-[13px] text-zinc-400"><b class="text-zinc-200">git 徽标</b>：绿色 = 工作区干净；琥珀色 <b>main ~3</b> = 分支 main 有 3 个文件未提交（含新文件）。不是 git 文件夹则不显示。点右上筛选「有未提交修改」可一次找出所有没收尾的活。</p>
            </div>
            <div class="flex gap-2.5">
              <span class="mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center"><span class="h-2 w-2 animate-pulse rounded-full bg-emerald-400"></span></span>
              <p class="text-[13px] text-zinc-400"><b class="text-zinc-200">绿色脉冲</b> = 该 AI 正在运行。点一下变红「确认停止？」，再点一次终止整个进程树（终端 + AI 一起）。AI 自己退出后按钮自动复原。</p>
            </div>
            <div class="flex gap-2.5">
              <Icon name="folder" class="mt-0.5 !h-4 !w-4 shrink-0 text-zinc-400" />
              <p class="text-[13px] text-zinc-400"><b class="text-zinc-200">悬浮操作行</b>（鼠标移上卡片出现）：浏览（应用内翻文件）· 时间机库（快照/回滚）· 编辑器打开 · 编辑 · 移除登记。</p>
            </div>
            <div class="flex gap-2.5">
              <Icon name="trash" class="mt-0.5 !h-4 !w-4 shrink-0 text-red-400/80" />
              <p class="text-[13px] text-zinc-400"><b class="text-zinc-200">放心删</b>：移除只删 Hangar 里的登记，<b>永远不动你的文件夹</b>。重新登记同一路径，历史自动复活。</p>
            </div>
          </section>
        </template>

        <!-- ================= AI 接入 ================= -->
        <template v-else>
          <section>
            <h3 class="mb-2 font-semibold text-zinc-100">让 AI 自己管机库（MCP）</h3>
            <p class="mb-3 text-[13px] text-zinc-400">
              在支持 MCP 的 AI（Claude Code / Codex / WorkBuddy…）里配置一次：
            </p>
            <pre class="overflow-x-auto rounded-xl border border-hangar-border bg-hangar-bg px-4 py-3 font-mono text-[12px] leading-relaxed text-zinc-300">claude mcp add hangar -- D:\...\hangar.exe mcp
# 或在 MCP 配置里：
{ "mcpServers": { "hangar": {
    "command": "hangar", "args": ["mcp"] } } }</pre>
            <p class="mt-3 text-[13px] text-zinc-400">
              连上后 AI 自动获得 7 个工具，且<b class="text-zinc-200">握手自带行为规则</b>——你不用教它：
            </p>
            <div class="mt-2 grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-[12.5px]">
              <code class="text-amber-300/90">upsert_workspace</code><span class="text-zinc-500">AI 建完新项目自动登记 + 打标签 + 归分组</span>
              <code class="text-amber-300/90">log_activity</code><span class="text-zinc-500">干完活自动写一笔工作汇报（进卡片和时间线）</span>
              <code class="text-amber-300/90">list / get</code><span class="text-zinc-500">问它"我有哪些空间 / 博客空间什么状态"</span>
              <code class="text-amber-300/90">open / launch</code><span class="text-zinc-500">"打开 X""在 X 里启动 claude" 直接办</span>
            </div>
          </section>

          <section class="rounded-xl border border-hangar-border p-4">
            <h3 class="mb-2 text-xs font-semibold uppercase tracking-wider text-zinc-500">也可以用命令行 / HTTP</h3>
            <pre class="overflow-x-auto font-mono text-[12px] leading-relaxed text-zinc-400">hangar ls / open demo / run demo / add …   # CLI，与界面同库
curl -H "Authorization: Bearer …" http://127.0.0.1:7333/workspaces   # REST</pre>
          </section>

          <section>
            <h3 class="mb-2 font-semibold text-zinc-100">数据与隐私</h3>
            <p class="text-[13px] text-zinc-400">
              所有数据在本机单个文件 <code class="text-zinc-300">%APPDATA%\hangar\hangar.db</code>
              ——零联网、零遥测、无账号。REST 仅监听 127.0.0.1 并要求 token。
              完整说明见 <code>docs/说明书.md</code>，技术细节见 <code>docs/GUIDE.zh-CN.md</code>。
            </p>
          </section>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.step-num {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border-radius: 999px;
  background: rgba(245, 165, 36, 0.15);
  color: #f5c063;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}
.chip-demo {
  border-radius: 999px;
  padding: 1px 8px;
  font-size: 11px;
  border: 1px solid #242730;
  background: rgba(39, 39, 42, 0.7);
  color: #a1a1aa;
}
.chip-demo.amber {
  background: rgba(245, 165, 36, 0.1);
  color: #fcd34d;
  border: none;
}
.chip-demo.git-dirty {
  font-family: Consolas, monospace;
  background: rgba(245, 165, 36, 0.1);
  color: #fbbf24;
  border: 1px solid rgba(245, 165, 36, 0.35);
}
.launch-demo {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 10px;
  border-radius: 8px;
  font-size: 12px;
  color: #d4d4d8;
  background: #1d2027;
  border: 1px solid #2a2d36;
}
.launch-demo.dashed {
  border-style: dashed;
}
.launch-demo :deep(svg) {
  color: #f5a524;
}
</style>
