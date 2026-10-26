mod browse;
mod db;
pub mod git;
pub mod launcher;
mod import;
mod model;
pub mod rest;
mod snapshot;

pub use browse::{list_dir, open_entry, read_text_file, write_text_file, FsEntry, TextFile};
pub use db::{activity_feed, FeedItem, HangarDb};
pub use git::{git_status, GitStatus};
pub use import::scan;
pub use model::{
    CandidateDir, FailedImport, Group, ImportSummary, LaunchConfig, NewGroup, NewLaunchConfig,
    NewWorkspace, QuickLaunch, Workspace, WorkspaceUpdate,
};
pub use snapshot::{
    delete_snapshot, list_snapshots, restore_snapshot, snapshot_before_launch, SnapInfo,
};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("该路径已登记过工作空间")]
    AlreadyRegistered,
    #[error("未找到工作空间")]
    NotFound,
    #[error("{0}")]
    Query(String),
    #[error("文件夹不存在或无法访问: {0}")]
    BadPath(String),
    #[error("数据库错误: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
