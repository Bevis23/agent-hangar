//! 目录导入扫描：在用户指定的根目录下（深度 ≤ 2）找出"像项目"的文件夹。
//! 判定依据是 marker 文件/目录（.git、package.json、Cargo.toml、CLAUDE.md…）。

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::model::CandidateDir;
use crate::{CoreError, Result};

const MARKERS: &[&str] = &[
    ".git",
    ".claude",
    ".vscode",
    ".idea",
    "CLAUDE.md",
    "AGENTS.md",
    "README.md",
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "composer.json",
    "Gemfile",
];

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "obj",
    "bin",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    ".cache",
    ".next",
    ".git",
    ".svn",
];

/// 扫描 root 下 1–2 层内的文件夹，跳过已登记路径。
/// all=false：只收"像项目"的（含 marker）；all=true：普通文件夹也收（排除垃圾目录和 . 开头目录）。
pub fn scan(root: &str, registered: &HashSet<String>, all: bool) -> Result<Vec<CandidateDir>> {
    let root_path =
        fs::canonicalize(root.trim()).map_err(|_| CoreError::BadPath(root.to_string()))?;
    let mut out = Vec::new();
    walk(&root_path, 0, registered, &mut out, all);
    // marker 多的排前（项目优先），其余按路径
    out.sort_by(|a, b| {
        b.markers
            .len()
            .cmp(&a.markers.len())
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(out)
}

fn walk(dir: &Path, depth: u32, registered: &HashSet<String>, out: &mut Vec<CandidateDir>, all: bool) {
    // all 模式：只扫当前层（直接子文件夹）；marker 模式：下探 2 层找嵌套项目
    let max_depth = if all { 1 } else { 2 };
    if depth > max_depth {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // 无权限等，静默跳过该层
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() || !ft.is_dir() {
            continue; // 跳过符号链接防环，只看目录
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else { continue };
        if SKIP_DIRS.contains(&name) {
            continue;
        }

        let markers = markers_of(&path);
        let take = if all {
            // 普通模式：全收，但 . 开头的配置目录（.git/.claude/.cache…）不算工作空间
            !name.starts_with('.')
        } else {
            !markers.is_empty()
        };
        if take {
            let norm = normalize(&path);
            if !registered.contains(&norm) {
                out.push(CandidateDir {
                    path: norm,
                    dir_name: name.to_string(),
                    markers,
                });
            }
        }
        walk(&path, depth + 1, registered, out, all);
    }
}

fn markers_of(dir: &Path) -> Vec<String> {
    MARKERS
        .iter()
        .filter(|m| dir.join(m).exists())
        .map(|m| m.to_string())
        .collect()
}

fn normalize(p: &Path) -> String {
    let canon = fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let mut s = canon.to_string_lossy().to_string();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_all_mode_includes_plain_folders() {
        let base = std::env::temp_dir().join(format!("hangar-imp-{}", std::process::id()));
        fs::create_dir_all(base.join("带git/inner")).unwrap();
        fs::write(base.join("带git/README.md"), "").unwrap();
        fs::create_dir_all(base.join("杂项-集合")).unwrap();
        fs::write(base.join("杂项-集合/note.txt"), "x").unwrap();
        fs::create_dir_all(base.join("node_modules")).unwrap();
        fs::create_dir_all(base.join(".claude")).unwrap();
        let empty = HashSet::new();

        // marker 模式：只有带 .gitignore（marker 表内）的
        let m = scan(base.to_str().unwrap(), &empty, false).unwrap();
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].dir_name, "带git");

        // all 模式：普通文件夹也收；node_modules 和 .claude 排除
        let a = scan(base.to_str().unwrap(), &empty, true).unwrap();
        let names: Vec<&str> = a.iter().map(|c| c.dir_name.as_str()).collect();
        assert!(names.contains(&"带git"));
        assert!(names.contains(&"杂项-集合"));
        assert!(!names.contains(&"node_modules"));
        assert!(!names.contains(&".claude"));
        // 带 marker 的排最前
        assert_eq!(a[0].dir_name, "带git");

        // 已登记的不再出现
        let mut reg = HashSet::new();
        reg.insert(a[0].path.clone());
        let a2 = scan(base.to_str().unwrap(), &reg, true).unwrap();
        assert!(!a2.iter().any(|c| c.dir_name == "带git"));

        fs::remove_dir_all(&base).unwrap();
    }
}
