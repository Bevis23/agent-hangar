# 录制后清理：移除演示卡片 + 删除演示文件夹
# 用法：powershell -ExecutionPolicy Bypass -File scripts\demo-clean.ps1

$ErrorActionPreference = "Continue"
$repo  = Split-Path -Parent $PSScriptRoot
$cli   = Join-Path $repo "target\debug\hangar.exe"
if (-not (Test-Path $cli)) { $cli = Join-Path $repo "target\release\hangar.exe" }

Get-Process -Name "hangar-app", "Hangar" -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 800

foreach ($name in @("博客演示", "API 服务", "设计素材", "会议记录", "实验田")) {
    & $cli rm $name 2>$null
}

$demoRoot = "D:\hangar-demo-data"
if (Test-Path $demoRoot) { Remove-Item $demoRoot -Recurse -Force }

Write-Output "✅ 已清理：演示卡片移除（软删除），D:\hangar-demo-data 已删除。"
Write-Output "   你自己的工作空间数据未受影响。"
