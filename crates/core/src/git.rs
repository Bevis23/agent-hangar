//! Git 基础状态查询：分支名 + 未提交文件数（含未跟踪）。
//! 只读、不会失败——非 git 目录返回 is_repo=false。

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub dirty: u32,
}

pub fn git_status(path: &str) -> GitStatus {
    let Ok(repo) = git2::Repository::open(path) else {
        return GitStatus { is_repo: false, branch: None, dirty: 0 };
    };
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(str::to_string));
    let dirty = count_dirty(&repo).unwrap_or(0);
    GitStatus { is_repo: true, branch, dirty }
}

fn count_dirty(repo: &git2::Repository) -> Result<u32, git2::Error> {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true);
    opts.exclude_submodules(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    Ok(statuses
        .iter()
        .filter(|e| e.status() != git2::Status::CURRENT)
        .count() as u32)
}

/// 最近 N 天的提交（时间戳, 首行摘要），新→旧，最多 limit 条。
/// 非 git 目录/无提交返回空。
pub fn recent_commits(path: &str, days: u32, limit: usize) -> Vec<(i64, String)> {
    let Ok(repo) = git2::Repository::open(path) else {
        return vec![];
    };
    let Ok(head) = repo.head() else { return vec![] };
    let Ok(mut walk) = repo.revwalk() else { return vec![] };
    let Some(head_oid) = head.target() else { return vec![] };
    if walk.push(head_oid).is_err() {
        return vec![];
    }
    let cutoff = now_secs() - (days as i64) * 86400;
    let mut out = Vec::new();
    for oid in walk.flatten() {
        let Ok(commit) = repo.find_commit(oid) else { continue };
        let ts = commit.time().seconds();
        if ts < cutoff {
            break; // revwalk 按时间新→旧，越过窗口即停
        }
        let summary = commit.summary().unwrap_or("").to_string();
        if summary.is_empty() {
            continue;
        }
        out.push((ts, summary));
        if out.len() >= limit {
            break;
        }
    }
    out
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
