use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::git::recent_commits;
use crate::model::{
    Group, LaunchConfig, NewLaunchConfig, NewWorkspace, QuickLaunch, Workspace, WorkspaceUpdate,
};
use crate::snapshot::list_snapshots;
use crate::{CoreError, Result};

const SCHEMA_V1: &str = r#"
CREATE TABLE IF NOT EXISTS groups (
  id   TEXT PRIMARY KEY,
  name TEXT NOT NULL UNIQUE,
  icon TEXT,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS workspaces (
  id             TEXT PRIMARY KEY,
  name           TEXT NOT NULL,
  path           TEXT NOT NULL UNIQUE,
  agent          TEXT,
  notes          TEXT,
  icon           TEXT,
  color          TEXT,
  group_id       TEXT REFERENCES groups(id),
  is_favorite    INTEGER NOT NULL DEFAULT 0,
  is_deleted     INTEGER NOT NULL DEFAULT 0,
  created_at     INTEGER NOT NULL,
  last_opened_at INTEGER
);

CREATE TABLE IF NOT EXISTS tags (
  id    TEXT PRIMARY KEY,
  name  TEXT NOT NULL UNIQUE,
  color TEXT
);

CREATE TABLE IF NOT EXISTS workspace_tags (
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  tag_id       TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (workspace_id, tag_id)
);

CREATE TABLE IF NOT EXISTS launch_configs (
  id           TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  name         TEXT NOT NULL,
  command      TEXT NOT NULL,
  shell        TEXT NOT NULL DEFAULT 'powershell',
  confirm_skip INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS open_events (
  id           INTEGER PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  opened_at    INTEGER NOT NULL,
  via          TEXT
);

CREATE INDEX IF NOT EXISTS idx_workspaces_deleted ON workspaces(is_deleted);
CREATE INDEX IF NOT EXISTS idx_open_events_ws ON open_events(workspace_id, opened_at);
"#;

pub struct HangarDb {
    conn: Connection,
}

impl HangarDb {
    /// 打开（或创建）%APPDATA%\hangar\hangar.db 并执行迁移。
    pub fn open() -> Result<Self> {
        let dir = dirs::data_dir()
            .ok_or_else(|| CoreError::BadPath("无法定位 AppData 目录".into()))?
            .join("hangar");
        std::fs::create_dir_all(&dir)?;
        let conn = Connection::open(dir.join("hangar.db"))?;
        // GUI 与 CLI 可能同时打开同一数据库：WAL + 5 秒等待，避免并发写入报 locked
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        // journal_mode 会返回结果行，用 query_row 读取；foreign_keys 不返回行。
        let _mode: String = conn.query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let mut db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> Result<()> {
        let version: i64 =
            self.conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            self.conn.execute_batch(SCHEMA_V1)?;
            self.conn.execute_batch("PRAGMA user_version=1")?;
        }
        if version < 2 {
            self.conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT);
                 PRAGMA user_version=2;",
            )?;
        }
        Ok(())
    }

    /// 全量读取设置（键值对）。外观/终端/编辑器等偏好。
    pub fn get_settings(&self) -> Result<HashMap<String, String>> {
        let mut stmt = self.conn.prepare("SELECT key, value FROM settings")?;
        let mut rows = stmt.query([])?;
        let mut out = HashMap::new();
        while let Some(row) = rows.next()? {
            out.insert(row.get(0)?, row.get(1)?);
        }
        Ok(out)
    }

    /// 批量写入设置（upsert）。
    pub fn set_settings(&self, entries: &HashMap<String, String>) -> Result<()> {
        for (k, v) in entries {
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![k, v],
            )?;
        }
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Workspace>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, agent, notes, icon, color, group_id, is_favorite,
                    created_at, last_opened_at
             FROM workspaces
             WHERE is_deleted = 0
             ORDER BY is_favorite DESC, COALESCE(last_opened_at, 0) DESC, created_at DESC",
        )?;
        let mut rows = stmt.query([])?;
        let mut result = Vec::new();
        while let Some(row) = rows.next()? {
            result.push(row_to_workspace(row)?);
        }
        self.attach_tags(&mut result)?;
        self.attach_launch_configs(&mut result)?;
        Ok(result)
    }

    pub fn get(&self, id: &str) -> Result<Workspace> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, path, agent, notes, icon, color, group_id, is_favorite,
                    created_at, last_opened_at
             FROM workspaces WHERE id = ?1 AND is_deleted = 0",
        )?;
        let mut rows = stmt.query(params![id])?;
        let mut ws = match rows.next()? {
            Some(row) => row_to_workspace(row)?,
            None => return Err(CoreError::NotFound),
        };
        self.attach_tags(std::slice::from_mut(&mut ws))?;
        self.attach_launch_configs(std::slice::from_mut(&mut ws))?;
        Ok(ws)
    }

    pub fn add(&self, n: NewWorkspace) -> Result<Workspace> {
        let name = n.name.trim();
        if name.is_empty() {
            return Err(CoreError::BadPath("名称不能为空".into()));
        }
        let path = normalize_path(&n.path)?;

        // 同路径处理：活跃记录 → 拒绝重复登记；软删除记录 → 复活并覆盖元数据
        // （path 列全局唯一，直接 INSERT 会撞约束）。
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM workspaces WHERE path = ?1 AND is_deleted = 1",
                params![path],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e),
            })?;
        if let Some(id) = existing {
            self.conn.execute(
                "UPDATE workspaces
                 SET is_deleted = 0, name = ?2, agent = ?3, notes = ?4, icon = ?5,
                     color = ?6, group_id = ?7
                 WHERE id = ?1",
                params![id, name, n.agent, n.notes, n.icon, n.color, n.group_id],
            )?;
            self.replace_tags(&id, &n.tags)?;
            return self.get(&id);
        }

        let alive: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM workspaces WHERE path = ?1 AND is_deleted = 0",
            params![path],
            |r| r.get(0),
        )?;
        if alive > 0 {
            return Err(CoreError::AlreadyRegistered);
        }

        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO workspaces (id, name, path, agent, notes, icon, color, group_id, is_favorite, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, ?9)",
            params![id, name, path, n.agent, n.notes, n.icon, n.color, n.group_id, now_secs()],
        )?;
        self.replace_tags(&id, &n.tags)?;
        self.replace_launch_configs(&id, &n.launch_configs)?;
        self.get(&id)
    }

    pub fn update(&self, u: WorkspaceUpdate) -> Result<Workspace> {
        let name = u.name.trim();
        if name.is_empty() {
            return Err(CoreError::BadPath("名称不能为空".into()));
        }
        let changed = self.conn.execute(
            "UPDATE workspaces SET name = ?2, agent = ?3, notes = ?4, icon = ?5, color = ?6, group_id = ?7
             WHERE id = ?1 AND is_deleted = 0",
            params![u.id, name, u.agent, u.notes, u.icon, u.color, u.group_id],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        self.replace_tags(&u.id, &u.tags)?;
        self.replace_launch_configs(&u.id, &u.launch_configs)?;
        self.get(&u.id)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE workspaces SET is_favorite = ?2 WHERE id = ?1 AND is_deleted = 0",
            params![id, favorite as i64],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    /// 软删除：只删登记记录的可见性，绝不触碰磁盘上的文件夹。
    pub fn soft_delete(&self, id: &str) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE workspaces SET is_deleted = 1 WHERE id = ?1 AND is_deleted = 0",
            params![id],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    pub fn record_open(&self, id: &str, via: &str) -> Result<()> {
        let now = now_secs();
        let changed = self.conn.execute(
            "UPDATE workspaces SET last_opened_at = ?2 WHERE id = ?1 AND is_deleted = 0",
            params![id, now],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        self.conn.execute(
            "INSERT INTO open_events (workspace_id, opened_at, via) VALUES (?1, ?2, ?3)",
            params![id, now, via],
        )?;
        Ok(())
    }

    /// 全部已登记路径（含软删除），供导入扫描排除。
    pub fn all_paths(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT path FROM workspaces")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(row.get(0)?);
        }
        Ok(out)
    }

    pub fn list_groups(&self) -> Result<Vec<Group>> {
        let mut stmt =
            self.conn.prepare("SELECT id, name, icon, sort_order FROM groups ORDER BY sort_order, name")?;
        let mut rows = stmt.query([])?;
        let mut out = Vec::new();
        while let Some(row) = rows.next()? {
            out.push(Group {
                id: row.get(0)?,
                name: row.get(1)?,
                icon: row.get(2)?,
                sort_order: row.get(3)?,
            });
        }
        Ok(out)
    }

    pub fn create_group(&self, name: &str, icon: Option<&str>) -> Result<Group> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::BadPath("分组名不能为空".into()));
        }
        let id = uuid::Uuid::now_v7().to_string();
        self.conn.execute(
            "INSERT INTO groups (id, name, icon) VALUES (?1, ?2, ?3)",
            params![id, name, icon],
        )?;
        Ok(Group {
            id,
            name: name.to_string(),
            icon: icon.map(str::to_string),
            sort_order: 0,
        })
    }

    /// 删除分组：先解除工作空间引用（schema 无 ON DELETE SET NULL，手动清），
    /// 工作空间本身不受影响，变为"未分组"。
    pub fn delete_group(&self, id: &str) -> Result<()> {
        self.conn
            .execute("UPDATE workspaces SET group_id = NULL WHERE group_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM groups WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn set_workspace_group(&self, workspace_id: &str, group_id: Option<&str>) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE workspaces SET group_id = ?2 WHERE id = ?1 AND is_deleted = 0",
            params![workspace_id, group_id],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    pub fn set_workspace_agent(&self, workspace_id: &str, agent: Option<&str>) -> Result<()> {
        let changed = self.conn.execute(
            "UPDATE workspaces SET agent = ?2 WHERE id = ?1 AND is_deleted = 0",
            params![workspace_id, agent],
        )?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    fn replace_tags(&self, workspace_id: &str, tags: &[String]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM workspace_tags WHERE workspace_id = ?1",
            params![workspace_id],
        )?;
        self.add_tags(workspace_id, tags)
    }

    /// 追加标签（CLI `hangar tag` 用；重复的自动忽略）。
    pub fn add_tags(&self, workspace_id: &str, tags: &[String]) -> Result<()> {
        for raw in tags {
            let tag = raw.trim();
            if tag.is_empty() {
                continue;
            }
            self.conn.execute(
                "INSERT OR IGNORE INTO tags (id, name) VALUES (?1, ?2)",
                params![uuid::Uuid::now_v7().to_string(), tag],
            )?;
            self.conn.execute(
                "INSERT OR IGNORE INTO workspace_tags (workspace_id, tag_id)
                 SELECT ?1, id FROM tags WHERE name = ?2",
                params![workspace_id, tag],
            )?;
        }
        Ok(())
    }

    fn attach_tags(&self, workspaces: &mut [Workspace]) -> Result<()> {
        if workspaces.is_empty() {
            return Ok(());
        }
        let mut stmt = self.conn.prepare(
            "SELECT wt.workspace_id, t.name
             FROM workspace_tags wt
             JOIN tags t ON t.id = wt.tag_id
             JOIN workspaces w ON w.id = wt.workspace_id AND w.is_deleted = 0
             ORDER BY t.name",
        )?;
        let mut rows = stmt.query([])?;
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        while let Some(row) = rows.next()? {
            let ws_id: String = row.get(0)?;
            let tag: String = row.get(1)?;
            map.entry(ws_id).or_default().push(tag);
        }
        for ws in workspaces.iter_mut() {
            ws.tags = map.remove(&ws.id).unwrap_or_default();
        }
        Ok(())
    }

    /// 整表替换该空间的启动配置（对话框全量保存语义）。空名称/空命令的行直接丢弃。
    fn replace_launch_configs(&self, workspace_id: &str, configs: &[NewLaunchConfig]) -> Result<()> {
        self.conn.execute(
            "DELETE FROM launch_configs WHERE workspace_id = ?1",
            params![workspace_id],
        )?;
        for c in configs {
            let name = c.name.trim();
            let command = c.command.trim();
            if name.is_empty() || command.is_empty() {
                continue;
            }
            self.conn.execute(
                "INSERT INTO launch_configs (id, workspace_id, name, command, shell, confirm_skip)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    uuid::Uuid::now_v7().to_string(),
                    workspace_id,
                    name,
                    command,
                    c.shell,
                    c.confirm_skip as i64
                ],
            )?;
        }
        Ok(())
    }

    fn attach_launch_configs(&self, workspaces: &mut [Workspace]) -> Result<()> {
        if workspaces.is_empty() {
            return Ok(());
        }
        let mut stmt = self.conn.prepare(
            "SELECT lc.id, lc.workspace_id, lc.name, lc.command, lc.shell, lc.confirm_skip
             FROM launch_configs lc
             JOIN workspaces w ON w.id = lc.workspace_id AND w.is_deleted = 0
             ORDER BY lc.rowid",
        )?;
        let mut rows = stmt.query([])?;
        let mut map: HashMap<String, Vec<LaunchConfig>> = HashMap::new();
        while let Some(row) = rows.next()? {
            let lc = LaunchConfig {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                name: row.get(2)?,
                command: row.get(3)?,
                shell: row.get(4)?,
                confirm_skip: row.get::<_, i64>(5)? != 0,
            };
            map.entry(lc.workspace_id.clone()).or_default().push(lc);
        }
        for ws in workspaces.iter_mut() {
            ws.launch_configs = map.remove(&ws.id).unwrap_or_default();
        }
        Ok(())
    }

    /// 按 id 取启动配置，同时带出工作空间目录（启动时需要）。
    pub fn get_launch_config(&self, id: &str) -> Result<(LaunchConfig, String)> {
        self.conn
            .query_row(
                "SELECT lc.id, lc.workspace_id, lc.name, lc.command, lc.shell, lc.confirm_skip, w.path
                 FROM launch_configs lc
                 JOIN workspaces w ON w.id = lc.workspace_id AND w.is_deleted = 0
                 WHERE lc.id = ?1",
                params![id],
                |r| {
                    Ok((
                        LaunchConfig {
                            id: r.get(0)?,
                            workspace_id: r.get(1)?,
                            name: r.get(2)?,
                            command: r.get(3)?,
                            shell: r.get(4)?,
                            confirm_skip: r.get::<_, i64>(5)? != 0,
                        },
                        r.get(6)?,
                    ))
                },
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound,
                e => CoreError::Db(e),
            })
    }

    /// 用户在确认弹窗勾选"记住选择"后持久化，之后该配置启动不再询问。
    pub fn set_launch_confirm_skip(&self, id: &str, skip: bool) -> Result<()> {
        let changed = self
            .conn
            .execute("UPDATE launch_configs SET confirm_skip = ?2 WHERE id = ?1", params![id, skip as i64])?;
        if changed == 0 {
            return Err(CoreError::NotFound);
        }
        Ok(())
    }

    /// 取分组：同名（不区分大小写）存在则返回，否则自动创建。
    pub fn ensure_group(&self, name: &str, icon: Option<&str>) -> Result<Group> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::BadPath("分组名不能为空".into()));
        }
        for g in self.list_groups()? {
            if g.name.to_lowercase() == name.to_lowercase() {
                return Ok(g);
            }
        }
        self.create_group(name, icon)
    }

    /// 模糊查找（精确名/路径 > 唯一包含匹配）。
    pub fn find_by_query(&self, query: &str) -> Result<Workspace> {
        let list = self.list()?;
        let q = query.trim().to_lowercase();
        if let Some(w) = list
            .iter()
            .find(|w| w.name.to_lowercase() == q || w.path.to_lowercase() == q)
        {
            return Ok(w.clone());
        }
        let hits: Vec<&Workspace> = list
            .iter()
            .filter(|w| w.name.to_lowercase().contains(&q) || w.path.to_lowercase().contains(&q))
            .collect();
        match hits.len() {
            0 => Err(CoreError::Query(format!("没有匹配「{query}」的工作空间"))),
            1 => Ok(hits[0].clone()),
            _ => Err(CoreError::Query(format!(
                "匹配到多个：{}；请用更精确的名称",
                hits.iter()
                    .map(|w| format!("{}（{}）", w.name, w.path))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ))),
        }
    }

    /// 追加一条工作动态到卡片备注（MCP log_activity / REST /log 共用）。
    pub fn log_activity(&self, query: &str, text: &str) -> Result<String> {
        let text = text.trim();
        if text.is_empty() {
            return Err(CoreError::BadPath("动态内容不能为空".into()));
        }
        let w = self.find_by_query(query)?;
        let old = w.notes.as_deref().unwrap_or("").trim_end();
        let notes = format!("{old}
[{}] {}", crate::snapshot::now_stamp(), text);
        self.update(WorkspaceUpdate {
            id: w.id.clone(),
            name: w.name.clone(),
            agent: w.agent.clone(),
            notes: Some(notes),
            icon: w.icon.clone(),
            color: w.color.clone(),
            group_id: w.group_id.clone(),
            launch_configs: w
                .launch_configs
                .iter()
                .map(|c| NewLaunchConfig {
                    name: c.name.clone(),
                    command: c.command.clone(),
                    shell: c.shell.clone(),
                    confirm_skip: c.confirm_skip,
                })
                .collect(),
            tags: w.tags.clone(),
        })?;
        Ok(format!("已记录动态到「{}」", w.name))
    }

    /// 快捷启动：同空间同名配置存在则更新命令/shell/confirm_skip，否则新建。
    /// 返回配置及其所在空间目录。
    pub fn upsert_launch_config(&self, q: &QuickLaunch) -> Result<(LaunchConfig, String)> {
        let name = q.name.trim();
        let command = q.command.trim();
        if name.is_empty() || command.is_empty() {
            return Err(CoreError::BadPath("启动配置名称/命令不能为空".into()));
        }
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM launch_configs WHERE workspace_id = ?1 AND name = ?2",
                params![q.workspace_id, name],
                |r| r.get(0),
            )
            .map(Some)
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                e => Err(e),
            })?;
        let cid = match existing {
            Some(id) => {
                self.conn.execute(
                    "UPDATE launch_configs SET command = ?2, shell = ?3, confirm_skip = ?4 WHERE id = ?1",
                    params![id, command, q.shell, q.confirm_skip as i64],
                )?;
                id
            }
            None => {
                let id = uuid::Uuid::now_v7().to_string();
                self.conn.execute(
                    "INSERT INTO launch_configs (id, workspace_id, name, command, shell, confirm_skip)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![id, q.workspace_id, name, command, q.shell, q.confirm_skip as i64],
                )?;
                id
            }
        };
        self.get_launch_config(&cid)
    }
}

fn row_to_workspace(row: &rusqlite::Row<'_>) -> std::result::Result<Workspace, rusqlite::Error> {
    Ok(Workspace {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        agent: row.get(3)?,
        notes: row.get(4)?,
        icon: row.get(5)?,
        color: row.get(6)?,
        group_id: row.get(7)?,
        is_favorite: row.get::<_, i64>(8)? != 0,
        created_at: row.get(9)?,
        last_opened_at: row.get(10)?,
        tags: Vec::new(),
        launch_configs: Vec::new(),
    })
}

/// 规范化路径：解析为绝对路径并去掉 Windows 的 \\?\ 前缀。
/// 文件夹不存在时返回 BadPath（登记前提：文件夹真实存在）。
fn normalize_path(p: &str) -> Result<String> {
    let trimmed = p.trim().trim_end_matches('\\').trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(CoreError::BadPath("路径不能为空".into()));
    }
    let canonical = std::fs::canonicalize(std::path::Path::new(trimmed))
        .map_err(|_| CoreError::BadPath(trimmed.to_string()))?;
    let mut s = canonical.to_string_lossy().to_string();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    Ok(s)
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// 活动时间线：把四路事件（起飞/打开/工作动态/git 提交/快照）汇成一条流。
pub fn activity_feed(db: &HangarDb, days: u32) -> Result<Vec<FeedItem>> {
    let cutoff = now_secs() - (days as i64) * 86400;
    let conn = &db.conn;
    let mut items: Vec<FeedItem> = Vec::new();

    // ① 打开/起飞事件
    let mut stmt = conn.prepare(
        "SELECT oe.opened_at, oe.via, w.id, w.name, w.agent
         FROM open_events oe JOIN workspaces w ON w.id = oe.workspace_id AND w.is_deleted = 0
         WHERE oe.opened_at >= ?1",
    )?;
    let mut rows = stmt.query(params![cutoff])?;
    while let Some(row) = rows.next()? {
        let ts: i64 = row.get(0)?;
        let via: String = row.get(1)?;
        let (kind, text) = if let Some(name) = via.strip_prefix("launch:") {
            ("launch", format!("起飞「{name}」"))
        } else {
            (
                "open",
                match via.as_str() {
                    "explorer" => "在资源管理器中打开".to_string(),
                    "terminal" => "打开终端".to_string(),
                    "editor" => "用编辑器打开".to_string(),
                    "cli" => "CLI 打开".to_string(),
                    "mcp" => "经 MCP 打开".to_string(),
                    other => format!("{other} 打开"),
                },
            )
        };
        items.push(FeedItem {
            ts,
            kind: kind.to_string(),
            ws_id: row.get(2)?,
            ws_name: row.get(3)?,
            agent: row.get(4)?,
            text,
        });
    }

    // ② 工作动态（log_activity 写进 notes 的时间戳行）
    let mut stmt = conn.prepare(
        "SELECT id, name, agent, notes FROM workspaces WHERE is_deleted = 0 AND notes IS NOT NULL",
    )?;
    let mut rows = stmt.query([])?;
    let mut note_rows: Vec<(String, String, Option<String>, String)> = Vec::new();
    while let Some(row) = rows.next()? {
        note_rows.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    }
    for (id, name, agent, notes) in note_rows {
        for line in notes.lines() {
            if let Some((ts, text)) = parse_note_line(line) {
                if ts >= cutoff {
                    items.push(FeedItem { ts, kind: "work".into(), ws_id: id.clone(), ws_name: name.clone(), agent: agent.clone(), text });
                }
            }
        }
    }

    // ③ 快照 + ④ git 提交（逐空间，最多扫 20 个最近活跃的）
    let mut stmt = conn.prepare(
        "SELECT id, name, agent, path FROM workspaces WHERE is_deleted = 0
         ORDER BY COALESCE(last_opened_at, created_at) DESC LIMIT 20",
    )?;
    let mut rows = stmt.query([])?;
    let mut ws_rows: Vec<(String, String, Option<String>, String)> = Vec::new();
    while let Some(row) = rows.next()? {
        ws_rows.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    }
    for (id, name, agent, path) in ws_rows {
        if let Ok(snaps) = list_snapshots(&id) {
            for s in snaps.iter().filter(|s| s.created_at >= cutoff) {
                items.push(FeedItem { ts: s.created_at, kind: "snapshot".into(), ws_id: id.clone(), ws_name: name.clone(), agent: agent.clone(), text: "起飞前自动快照".into() });
            }
        }
        for (ts, summary) in recent_commits(&path, days, 10) {
            items.push(FeedItem { ts, kind: "commit".into(), ws_id: id.clone(), ws_name: name.clone(), agent: agent.clone(), text: summary });
        }
    }

    items.sort_by(|a, b| b.ts.cmp(&a.ts));
    items.truncate(300);
    Ok(items)
}

/// 活动时间线的一条聚合事件（四源：起飞/打开/工作动态/git 提交）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedItem {
    pub ts: i64,
    /// launch | open | work | commit | snapshot
    pub kind: String,
    pub ws_id: String,
    pub ws_name: String,
    pub agent: Option<String>,
    pub text: String,
}

/// 解析 log_activity 写入的动态行："[2026-10-25 19:27] 干了什么" → (epoch, 文本)。
/// 时间按写入时区 UTC+8 计算。
fn parse_note_line(line: &str) -> Option<(i64, String)> {
    let rest = line.trim().strip_prefix('[')?;
    let close = rest.find(']')?;
    let stamp = &rest[..close];
    let text = rest[close + 1..].trim().to_string();
    if text.is_empty() {
        return None;
    }
    let mut parts = stamp.split_whitespace();
    let date = parts.next()?;
    let time = parts.next()?;
    let mut d = date.split('-');
    let y: i64 = d.next()?.parse().ok()?;
    let m: u32 = d.next()?.parse().ok()?;
    let day: u32 = d.next()?.parse().ok()?;
    let mut t = time.split(':');
    let hh: i64 = t.next()?.parse().ok()?;
    let mm: i64 = t.next()?.parse().ok()?;
    // days_from_civil（Howard Hinnant 逆向）
    let z = y - if m <= 2 { 1 } else { 0 };
    let era = if z >= 0 { z } else { z - 399 } / 400;
    let yoe = (z - era * 400) as i64;
    let doy = ((153 * (if m > 2 { m as i64 - 3 } else { m as i64 + 9 }) + 2) / 5) + (day as i64 - 1);
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    let epoch = days * 86400 + (hh - 8) * 3600 + mm * 60;
    Some((epoch, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_line_parse_and_civil_roundtrip() {
        let (ts, text) = parse_note_line("[2026-10-25 19:27] 重构了登录页").unwrap();
        // 2026-10-25 19:27 UTC+8 → 11:27 UTC
        assert_eq!(text, "重构了登录页");
        let days = ts.div_euclid(86400);
        let secs = ts.rem_euclid(86400);
        // civil_from_days 逆向校验日期
        let z = days + 719468;
        let era = if z >= 0 { z } else { z - 146096 } / 146097;
        let doe = (z - era * 146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        assert_eq!((y, m, d), (2026, 10, 25));
        assert_eq!(secs, 11 * 3600 + 27 * 60);
        assert!(parse_note_line("普通备注行").is_none());
        assert!(parse_note_line("[坏数据]  ").is_none());
    }
}
