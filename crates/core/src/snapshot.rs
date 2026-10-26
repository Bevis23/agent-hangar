//! 时间机库：起飞前自动快照 + 恢复。
//! 每次 agent 起飞前把工作空间打包成 zip（排除 node_modules 等重型目录），
//! 每空间保留最近 5 份；AI 改坏了随时一键回滚。非 git 目录同样有效。

use serde::Serialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::{CoreError, Result};

/// 与 import::SKIP_DIRS 同源的排除清单（重型/生成物目录）。
const SKIP_DIRS: &[&str] = &[
    "node_modules", "target", "dist", "build", "out", "obj", ".venv", "venv", "env",
    "__pycache__", ".cache", ".next", ".git", ".svn",
];

/// 快照体积/数量上限：超出则本次静默跳过（不阻塞起飞）。
const MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_FILES: usize = 4000;
const KEEP_PER_WORKSPACE: usize = 5;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapInfo {
    /// zip 文件名（纯数字时间戳.zip）
    pub file: String,
    pub created_at: i64,
    pub size: u64,
}

fn snap_dir(workspace_id: &str) -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("hangar")
        .join("snapshots")
        .join(workspace_id)
}

/// 起飞前快照。enabled 由调用方从设置解析（默认开）。
/// 最佳努力：任何失败都返回 Ok(None)，绝不阻塞起飞。
pub fn snapshot_before_launch(workspace_id: &str, path: &str, enabled: bool) -> Result<Option<SnapInfo>> {
    if !enabled {
        return Ok(None);
    }
    match try_snapshot(workspace_id, path) {
        Ok(info) => Ok(info),
        Err(_) => Ok(None),
    }
}

fn try_snapshot(workspace_id: &str, path: &str) -> Result<Option<SnapInfo>> {
    let src = Path::new(path);
    if !src.is_dir() {
        return Ok(None);
    }
    let dir = snap_dir(workspace_id);
    fs::create_dir_all(&dir)?;
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let zip_path = dir.join(format!("{secs:010}.zip"));

    let file = fs::File::create(&zip_path)?;
    let mut zw = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default();

    let root = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    let mut total_bytes: u64 = 0;
    let mut count: usize = 0;
    let mut overflow = false;

    for entry in walkdir::WalkDir::new(&root)
        .into_iter()
        // 剪枝：命中排除名单的目录整棵不下钻（根目录除外）
        .filter_entry(|e| {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !(e.file_type().is_dir() && SKIP_DIRS.contains(&name.as_ref()))
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        // 相对路径里的任一段命中排除名单则跳过（双保险）
        let rel = match entry.path().strip_prefix(&root) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if rel.split('/').any(|seg| SKIP_DIRS.contains(&seg)) {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        total_bytes += meta.len();
        count += 1;
        if total_bytes > MAX_TOTAL_BYTES || count > MAX_FILES {
            overflow = true;
            break;
        }
        let Ok(data) = fs::read(entry.path()) else { continue };
        if zw.start_file(rel.as_str(), options).is_err() {
            overflow = true;
            break;
        }
        if zw.write_all(&data).is_err() {
            overflow = true;
            break;
        }
    }
    let _ = zw.finish();

    if overflow {
        // 超限：不留半成品，本次不快照
        let _ = fs::remove_file(&zip_path);
        return Ok(None);
    }
    if count == 0 {
        let _ = fs::remove_file(&zip_path);
        return Ok(None);
    }

    prune(workspace_id, KEEP_PER_WORKSPACE)?;
    Ok(Some(SnapInfo {
        file: zip_path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        created_at: secs as i64,
        size: total_bytes,
    }))
}

/// 只保留最近 keep 份（文件名是零填充时间戳，字典序即时间序）。
fn prune(workspace_id: &str, keep: usize) -> Result<()> {
    let dir = snap_dir(workspace_id);
    let mut zips: Vec<PathBuf> = fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x == "zip").unwrap_or(false))
        .collect();
    zips.sort();
    while zips.len() > keep {
        let victim = zips.remove(0);
        let _ = fs::remove_file(victim);
    }
    Ok(())
}

/// 某空间的快照列表（新→旧）。
pub fn list_snapshots(workspace_id: &str) -> Result<Vec<SnapInfo>> {
    let dir = snap_dir(workspace_id);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for e in fs::read_dir(&dir)?.flatten() {
        let path = e.path();
        if path.extension().map(|x| x == "zip").unwrap_or(false) {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let created_at = name
                .trim_end_matches(".zip")
                .parse::<i64>()
                .unwrap_or(0);
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            out.push(SnapInfo { file: name, created_at, size });
        }
    }
    out.sort_by(|a, b| b.file.cmp(&a.file));
    Ok(out)
}

/// 恢复快照：把 zip 内容覆盖解压回工作空间。返回恢复的文件数。
/// 防路径穿越：条目名含 ".." 或绝对路径的一律跳过。
pub fn restore_snapshot(workspace_id: &str, file: &str, ws_path: &str) -> Result<u64> {
    if file.contains("..") || !file.ends_with(".zip") {
        return Err(CoreError::BadPath(file.to_string()));
    }
    let zip_path = snap_dir(workspace_id).join(file);
    let f = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(f).map_err(|e| CoreError::Io(std::io::Error::other(format!("读取快照失败: {e}"))))?;
    let dest_root = fs::canonicalize(ws_path)
        .map_err(|_| CoreError::BadPath(ws_path.to_string()))?;
    let mut restored: u64 = 0;
    for i in 0..archive.len() {
        let mut zf = archive.by_index(i).map_err(|e| CoreError::Io(std::io::Error::other(format!("读取快照条目失败: {e}"))))?;
        let name = zf.name().to_string();
        if name.starts_with('/') || name.starts_with('\\') || name.split('/').any(|s| s == "..") {
            continue;
        }
        let target = dest_root.join(&name);
        if zf.is_dir() {
            let _ = fs::create_dir_all(&target);
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&target)?;
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf)?;
        out.write_all(&buf)?;
        restored += 1;
    }
    Ok(restored)
}

/// 删除一份快照。
pub fn delete_snapshot(workspace_id: &str, file: &str) -> Result<()> {
    if file.contains("..") || !file.ends_with(".zip") {
        return Err(CoreError::BadPath(file.to_string()));
    }
    fs::remove_file(snap_dir(workspace_id).join(file))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_restore_roundtrip() {
        // 用临时目录当"工作空间"和快照存储（直接测 try_snapshot 会写真实 AppData，
        // 因此这里绕过 enabled 开关，手工构造：先造空间，快照，改文件，恢复，比对）
        let ws = std::env::temp_dir().join(format!("hangar-snap-ws-{}", std::process::id()));
        fs::create_dir_all(ws.join("sub")).unwrap();
        fs::write(ws.join("README.md"), "# 原始内容").unwrap();
        fs::write(ws.join("sub/code.py"), "print(1)").unwrap();
        fs::create_dir_all(ws.join("node_modules")).unwrap(); // 应被排除

        // 快照（直接调用 try_snapshot 会写入真实 AppData 目录，测试里用独立 workspace id
        // 隔离，结束后删除该 id 的快照目录）
        let id = format!("test-{}", std::process::id());
        let info = try_snapshot(&id, ws.to_str().unwrap()).unwrap().expect("应有快照");
        assert!(info.size > 0);
        assert_eq!(list_snapshots(&id).unwrap().len(), 1);

        // "AI 改坏"文件 + 删除另一个
        fs::write(ws.join("README.md"), "# 被改坏的内容").unwrap();
        fs::remove_file(ws.join("sub/code.py")).unwrap();

        let n = restore_snapshot(&id, &info.file, ws.to_str().unwrap()).unwrap();
        assert!(n >= 2);
        assert_eq!(fs::read_to_string(ws.join("README.md")).unwrap(), "# 原始内容");
        assert!(ws.join("sub/code.py").exists());
        // node_modules 不在快照里，恢复也不会凭空造出来
        assert!(ws.join("node_modules").is_dir()); // 原有的仍在

        delete_snapshot(&id, &info.file).unwrap();
        assert!(list_snapshots(&id).unwrap().is_empty());
        fs::remove_dir_all(&ws).unwrap();
        let _ = fs::remove_dir_all(snap_dir(&id));
    }
}

/// 当前时间戳（UTC+8）："YYYY-MM-DD HH:MM"，与 log_activity 写入格式一致。
pub fn now_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 8 * 3600;
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 { y += 1; }
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, (rem % 3600) / 60)
}
