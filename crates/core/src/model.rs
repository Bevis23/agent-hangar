use serde::{Deserialize, Serialize};

/// 一个已登记的工作空间（对应磁盘上的一个文件夹）。
/// serde 字段命名为 camelCase，与前端 TypeScript 类型一一对应。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub agent: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub group_id: Option<String>,
    pub is_favorite: bool,
    pub created_at: i64,
    pub last_opened_at: Option<i64>,
    pub tags: Vec<String>,
    pub launch_configs: Vec<LaunchConfig>,
}

/// 一条 agent 启动配置：在某个 shell 里、该工作空间目录下执行 command。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchConfig {
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub command: String,
    pub shell: String,
    pub confirm_skip: bool,
}

/// 新建/替换启动配置的载荷（保存采用整表替换，id 由后端生成）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewLaunchConfig {
    pub name: String,
    pub command: String,
    pub shell: String,
    #[serde(default)]
    pub confirm_skip: bool,
}

/// 卡片「启动…」菜单的快捷启动载荷：不存在同名配置则创建，存在则更新命令。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickLaunch {
    pub workspace_id: String,
    pub name: String,
    pub command: String,
    pub shell: String,
    #[serde(default)]
    pub confirm_skip: bool,
}

/// 新增工作空间的载荷。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewWorkspace {
    pub name: String,
    pub path: String,
    pub agent: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub group_id: Option<String>,
    #[serde(default)]
    pub launch_configs: Vec<NewLaunchConfig>,
    pub tags: Vec<String>,
}

/// 编辑工作空间的载荷：全量可编辑字段（对话框提交所有字段，不做部分 patch）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceUpdate {
    pub id: String,
    pub name: String,
    pub agent: Option<String>,
    pub notes: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub group_id: Option<String>,
    #[serde(default)]
    pub launch_configs: Vec<NewLaunchConfig>,
    pub tags: Vec<String>,
}

/// 分组。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub sort_order: i64,
}

/// 新建分组的载荷。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewGroup {
    pub name: String,
    pub icon: Option<String>,
}

/// 目录导入扫描的一个候选文件夹。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidateDir {
    pub path: String,
    pub dir_name: String,
    pub markers: Vec<String>,
}

/// 批量导入结果。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub added: Vec<Workspace>,
    pub failed: Vec<FailedImport>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedImport {
    pub path: String,
    pub reason: String,
}
