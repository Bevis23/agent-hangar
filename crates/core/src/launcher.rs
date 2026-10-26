//! 终端与 agent 启动（Windows）。
//! 设计原则：用户不需要知道也不需要关心终端——
//! 统一走 Windows Terminal（无则 PowerShell），agent 命令照常运行。不涉及 Git。

use std::process::{Child, Command, Stdio};

/// 终端类子进程的输出重定向到临时文件：
/// 不能继承调用方管道（CLI/MCP 管道场景会被握住挂死）；stdin 保持默认继承。
pub(crate) fn detached(cmd: &mut Command) -> &mut Command {
    let open = || {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(std::env::temp_dir().join("hangar-terminals.log"))
            .ok()
    };
    if let (Some(out), Some(err)) = (open(), open()) {
        cmd.stdout(Stdio::from(out)).stderr(Stdio::from(err));
    }
    cmd
}

const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;

fn open_powershell(dir: &str) -> std::io::Result<Child> {
    use std::os::windows::process::CommandExt;
    let mut c = Command::new("powershell");
    detached(&mut c).creation_flags(CREATE_NEW_CONSOLE).args([
        "-NoExit",
        "-NoLogo",
        "-Command",
        &format!("Set-Location -LiteralPath '{dir}'"),
    ]);
    c.spawn()
}

/// 经 `cmd /c start /wait` 启动 PowerShell 并保持窗口：
/// start 给子进程全新控制台与句柄——直接 spawn 会继承调用方 stdin（EOF 时
/// powershell 跑完命令即退，表现为窗口闪一下就没）。实测此法从最恶劣环境（无控制台
/// 的自动化进程）拉起也稳定存活。
///
/// `/wait` 让 cmd 驻留到终端窗口关闭为止，因此返回的 Child 就是真实可跟踪的句柄：
/// - `try_wait` 反映窗口是否还在（运行状态不再永远秒变 stopped）；
/// - `taskkill /PID <pid> /T /F` 能沿 cmd → powershell → agent 杀掉整棵进程树。
/// 命令里的双引号统一转为单引号（PowerShell 等价），避免与 cmd 的外层引号冲突；
/// 引号开头的命令视为"带引号路径的 exe"，前置 & 调用符。
fn launch_via_start(dir: &str, command: &str) -> std::io::Result<Child> {
    let cmd = command.replace('"', "'");
    let stmt = if cmd.starts_with('\'') {
        format!("& {cmd}")
    } else {
        cmd
    };
    let mut c = Command::new("cmd");
    detached(&mut c).args([
        "/c",
        "start",
        "",
        "/wait",
        "powershell",
        "-NoExit",
        "-NoLogo",
        "-Command",
        &format!("Set-Location -LiteralPath '{dir}'; {stmt}"),
    ]);
    c.spawn()
}

/// 在终端中打开目录（不运行命令）：wt 优先（标签页体验），回退独立 PowerShell。
pub fn open_terminal_in(dir: &str) -> std::io::Result<()> {
    let mut c = Command::new("wt");
    detached(&mut c).arg("-d").arg(dir);
    if c.spawn().is_ok() {
        return Ok(());
    }
    open_powershell(dir).map(|_| ())
}

/// 在新窗口里、指定目录下执行启动命令；命令退出后窗口保持打开。
/// 返回的 Child 对应驻留的 cmd 进程，窗口存活期间它一直运行（见 launch_via_start）。
pub fn spawn_in_shell(dir: &str, command: &str) -> std::io::Result<Child> {
    launch_via_start(dir, command)
}
