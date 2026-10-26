//! 应用内文件浏览：列出目录内容（只读）。
//! 📂 打开不再跳出应用，而是在 Hangar 内的抽屉里浏览；
//! 打开单个文件/文件夹交给系统默认程序（经 start，全新句柄）。

use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

use crate::launcher::detached;
use crate::{CoreError, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FsEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: i64,
}

/// 列出目录：文件夹在前，其余按名称不区分大小写排序。
/// 隐藏文件也列出（这是开发者的工具，.git/.claude 恰恰是重点）。
pub fn list_dir(path: &str) -> Result<Vec<FsEntry>> {
    let dir = fs::canonicalize(path).map_err(|_| CoreError::BadPath(path.to_string()))?;
    if !dir.is_dir() {
        return Err(CoreError::BadPath(path.to_string()));
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)?.flatten() {
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = meta.is_dir();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        out.push(FsEntry {
            name,
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
            modified,
        });
    }
    out.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(out)
}

/// 规范化路径为"人类可读"形式：canonicalize 产生的 \\?\ 前缀和尾部分隔符
/// 交给 cmd/系统 API 会报"找不到文件"（0x80070002），必须剥掉。
fn display_path(p: &Path) -> String {
    let mut s = p.to_string_lossy().to_string();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    while s.ends_with('\\') || s.ends_with('/') {
        s.pop();
    }
    s
}

/// 用系统默认程序打开一个文件；文件夹则用资源管理器显示。
/// 经 `cmd /c start`（全新句柄，不继承任何东西）。
pub fn open_entry(path: &str) -> Result<()> {
    let p = fs::canonicalize(path).map_err(|_| CoreError::BadPath(path.to_string()))?;
    if !p.exists() {
        return Err(CoreError::BadPath(path.to_string()));
    }
    let display = display_path(&p);
    let arg = if p.is_dir() {
        // 资源管理器"显示该文件夹"
        format!("explorer,\"{display}\"")
    } else {
        format!("\"{display}\"")
    };
    let mut c = Command::new("cmd");
    detached(&mut c).args(["/c", "start", "", &arg]);
    c.spawn().map_err(CoreError::Io)?;
    Ok(())
}

/// 单个文本文件的内容（read_text_file 返回）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFile {
    pub content: String,
    pub truncated: bool,
    pub size: u64,
}

const MAX_TEXT: u64 = 2 * 1024 * 1024; // 2MB 上限，超出截断（查看器场景足够）

/// 读取文本文件（UTF-8，非法字节用替换符）。
pub fn read_text_file(path: &str) -> Result<TextFile> {
    let p = Path::new(path);
    let meta = fs::metadata(p).map_err(|_| CoreError::BadPath(path.to_string()))?;
    if meta.is_dir() {
        return Err(CoreError::BadPath(path.to_string()));
    }
    let file = fs::File::open(p).map_err(CoreError::Io)?;
    let cap = MAX_TEXT.min(meta.len()) as usize;
    use std::io::Read;
    let mut bytes = Vec::with_capacity(cap);
    (&file)
        .take(cap.try_into().unwrap_or(u64::MAX))
        .read_to_end(&mut bytes)
        .map_err(CoreError::Io)?;
    Ok(TextFile {
        content: String::from_utf8_lossy(&bytes).to_string(),
        truncated: meta.len() > MAX_TEXT,
        size: meta.len(),
    })
}

/// 保存文本文件（UTF-8 覆盖写）。
pub fn write_text_file(path: &str, content: &str) -> Result<()> {
    fs::write(path, content).map_err(CoreError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_dir_sorted_dirs_first_and_sizes() {
        let base = std::env::temp_dir().join(format!("hangar-test-{}", std::process::id()));
        let sub = base.join("zsub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(base.join("a.txt"), "hello").unwrap();
        fs::write(base.join("中文.md"), "内容").unwrap();

        let entries = list_dir(base.to_str().unwrap()).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(entries[0].is_dir && entries[0].name == "zsub");
        let a = entries.iter().find(|e| e.name == "a.txt").unwrap();
        assert_eq!(a.size, 5);
        let zh = entries.iter().find(|e| e.name == "中文.md").unwrap();
        assert_eq!(zh.size, 6); // UTF-8 中文 3 字节 × 2

        // open_entry 对不存在的路径报错，不 panic
        assert!(open_entry(base.join("nope.txt").to_str().unwrap()).is_err());

        // read/write 往返：写入 UTF-8 中文读回一致
        let f = base.join("round.md");
        write_text_file(f.to_str().unwrap(), "# 标题\n\n你好 Hangar\n").unwrap();
        let tf = read_text_file(f.to_str().unwrap()).unwrap();
        assert!(!tf.truncated);
        assert!(tf.content.contains("你好 Hangar"));

        fs::remove_dir_all(&base).unwrap();
    }
}
