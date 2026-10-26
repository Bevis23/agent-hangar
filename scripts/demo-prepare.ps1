# 录制演示脚本：demo-prepare.ps1 / demo-clean.ps1
# 用途：为 README 首图 GIF 录制准备干净的演示数据，录完一键清理。
# 用法：
#   录制前：  powershell -ExecutionPolicy Bypass -File scripts\demo-prepare.ps1
#   录制后：  powershell -ExecutionPolicy Bypass -File scripts\demo-clean.ps1
# 前提：Hangar 仓库在本路径（脚本自动定位 CLI），桌面端 Hangar 已关闭。

$ErrorActionPreference = "Stop"

# ---- 定位 ----
$repo  = Split-Path -Parent $PSScriptRoot
$cli   = Join-Path $repo "target\debug\hangar.exe"
if (-not (Test-Path $cli)) { $cli = Join-Path $repo "target\release\hangar.exe" }
if (-not (Test-Path $cli)) { throw "未找到 hangar.exe，请先 cargo build -p hangar-cli" }

$demoRoot = "D:\hangar-demo-data"

# ---- 关闭正在运行的 Hangar（避免数据库占用）----
Get-Process -Name "hangar-app", "Hangar" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800

# ---- 1. 造演示文件夹 ----
if (Test-Path $demoRoot) { Remove-Item $demoRoot -Recurse -Force }
$dirs = @{
    "blog-demo"     = "git"
    "api-server"    = "git"
    "设计素材"       = "plain"
    "会议记录"       = "plain"
    "experiments"   = "plain"
}
foreach ($name in $dirs.Keys) {
    $p = Join-Path $demoRoot $name
    New-Item -ItemType Directory -Force -Path $p | Out-Null
    Set-Content -Path (Join-Path $p "README.md") -Value "# $name（演示用）" -Encoding UTF8
    if ($dirs[$name] -eq "git") {
        Push-Location $p
        git init -q
        git add .
        git commit -qm "init"
        Pop-Location
    }
}
# 做一个"被 AI 改坏"的状态：api-server 改一个文件不提交（卡片上会出现 ~1）
Set-Content -Path (Join-Path $demoRoot "api-server\config.json") -Value '{ "debug": true, "broken": true }' -Encoding UTF8

# ---- 2. 登记进机库（带分组/标签/agent）----
& $cli add (Join-Path $demoRoot "blog-demo")  --name "博客演示"   --agent claude-code --tag 写作 --tag 演示 --group 内容创作
& $cli add (Join-Path $demoRoot "api-server") --name "API 服务"   --agent codex       --tag 后端 --tag 演示 --group 开发
& $cli add (Join-Path $demoRoot "设计素材")    --name "设计素材"   --agent workbuddy   --tag 设计            --group 内容创作
& $cli add (Join-Path $demoRoot "会议记录")    --name "会议记录"   --agent manual                            --group 工作日常
& $cli add (Join-Path $demoRoot "experiments") --name "实验田"     --agent claude-code --tag 实验 --tag 演示  --group 开发

# ---- 3. 收尾提示 ----
Write-Output ""
Write-Output "✅ 演示环境就绪："
Write-Output "   - 演示文件夹：$demoRoot（5 个，其中 api-server 是 git 且有 1 个未提交文件）"
Write-Output "   - 机库里新增 5 张卡片，分属 3 个分组（内容创作/开发/工作日常）"
Write-Output ""
Write-Output "录制建议：打开 Hangar → 按 F11 全屏 → 开始录制。录完跑 demo-clean.ps1 清理。"
