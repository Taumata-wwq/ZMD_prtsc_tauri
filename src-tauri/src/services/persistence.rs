use rusqlite::{params, Connection, OptionalExtension};
use std::sync::Mutex;
use tauri::{AppHandle, Manager};

use crate::error::{AppError, AppResult};
use crate::models::region::RegionConfig;
use crate::models::scroll_mode::{ScrollMode, SCROLL_MODES};
use crate::models::session::CaptureSession;
use crate::models::setting::{default_regions, default_settings, WindowState};

/// 从 rusqlite::Row 构造 RegionConfig
/// 统一处理 19 个字段的读取，避免重复
fn row_to_region_config(row: &rusqlite::Row<'_>) -> rusqlite::Result<RegionConfig> {
    Ok(RegionConfig {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        category: row.get(2)?,
        aspect_ratio: row.get(3)?,
        scroll_mode: row.get(4)?,
        grid_rows: row.get(5)?,
        grid_cols: row.get(6)?,
        overlap_x: row.get(7)?,
        overlap_y: row.get(8)?,
        drag_x: row.get(9)?,
        drag_y: row.get(10)?,
        capture_region_x: row.get(11)?,
        capture_region_y: row.get(12)?,
        capture_offset_y: row.get(13)?,
        template_ref: row.get(14)?,
        target_w: row.get(15)?,
        target_h: row.get(16)?,
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
    })
}

/// 数据库连接状态（Tauri managed state）
pub struct DbState {
    pub conn: Mutex<Connection>,
}

impl DbState {
    /// 锁数据库连接的辅助方法
    /// 统一处理 Mutex lock 错误，避免 25 处重复代码
    fn lock_conn(&self) -> AppResult<std::sync::MutexGuard<'_, rusqlite::Connection>> {
        self.conn.lock().map_err(|e| AppError::new(format!("锁连接失败: {}", e), "LOCK_ERROR"))
    }

    /// 在 Tauri setup 阶段初始化数据库
    pub fn init(app: &AppHandle) -> AppResult<Self> {
        let app_data_dir = app.path().app_data_dir()
            .map_err(|e| AppError::new(format!("无法获取 app_data_dir: {}", e), "PATH_ERROR"))?;
        std::fs::create_dir_all(&app_data_dir)
            .map_err(|e| AppError::new(format!("创建 app_data_dir 失败: {}", e), "IO_ERROR"))?;

        let db_path = app_data_dir.join("zmd.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| AppError::new(format!("打开数据库失败: {}", e), "SQLITE_ERROR"))?;

        // 启用外键约束
        conn.execute("PRAGMA foreign_keys = ON;", [])?;

        // 创建输出目录
        let output_dir = app_data_dir.join("output");
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| AppError::new(format!("创建输出目录失败: {}", e), "IO_ERROR"))?;

        let state = Self {
            conn: Mutex::new(conn),
        };

        state.create_tables()?;
        state.migrate_region_config()?;
        state.migrate_capture_region()?;
        state.migrate_cleanup_redundant_modes()?;
        state.migrate_add_target_columns()?;
        state.seed_defaults()?;
        Ok(state)
    }

    /// 创建所有表
    fn create_tables(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;

        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS region_config (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                aspect_ratio TEXT NOT NULL,
                scroll_mode TEXT NOT NULL,
                grid_rows INTEGER NOT NULL,
                grid_cols INTEGER NOT NULL,
                overlap_x REAL NOT NULL,
                overlap_y REAL NOT NULL,
                drag_x INTEGER NOT NULL,
                drag_y INTEGER NOT NULL,
                capture_region_x REAL NOT NULL DEFAULT 0.0,
                capture_region_y REAL NOT NULL DEFAULT 0.0,
                capture_offset_y INTEGER NOT NULL DEFAULT 0,
                template_ref TEXT,
                target_w INTEGER NOT NULL DEFAULT 0,
                target_h INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(name, aspect_ratio, scroll_mode)
            );

            CREATE TABLE IF NOT EXISTS scroll_mode (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                scroll_count INTEGER NOT NULL,
                is_default BOOLEAN NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS capture_session (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                region TEXT,
                scroll_mode TEXT,
                grid_rows INTEGER,
                grid_cols INTEGER,
                total_shots INTEGER,
                status TEXT NOT NULL,
                original_path TEXT,
                exported_path TEXT,
                crop_box TEXT,
                output_format TEXT,
                jpg_quality INTEGER
            );

            CREATE TABLE IF NOT EXISTS app_setting (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS window_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                x INTEGER,
                y INTEGER,
                width INTEGER,
                height INTEGER,
                is_maximized BOOLEAN DEFAULT 0,
                always_on_top BOOLEAN DEFAULT 0,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_session_started ON capture_session(started_at DESC);
        ")?;

        Ok(())
    }

    /// 兼容旧版本数据库：检查并添加 region_config 缺失的列
    /// 使用 PRAGMA table_info 查询现有列名，对缺失列执行 ALTER TABLE ADD COLUMN
    /// 必须支持幂等（多次执行不报错）
    fn migrate_region_config(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;

        // 收集 region_config 现有列名
        let mut stmt = conn.prepare("PRAGMA table_info(region_config)")?;
        let existing_cols: std::collections::HashSet<String> = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            Ok(name)
        })?.filter_map(|r| r.ok()).collect();
        drop(stmt);

        // 新增列定义：(列名, 列类型 + 约束)
        // SQLite 的 ALTER TABLE ADD COLUMN 对 NOT NULL 列必须提供 DEFAULT
        let new_cols: &[(&str, &str)] = &[
            ("capture_region_x", "REAL NOT NULL DEFAULT 0.0"),
            ("capture_region_y", "REAL NOT NULL DEFAULT 0.0"),
            ("capture_offset_y", "INTEGER NOT NULL DEFAULT 0"),
            ("template_ref", "TEXT"),
        ];

        for (col_name, col_def) in new_cols {
            if !existing_cols.contains(*col_name) {
                let sql = format!("ALTER TABLE region_config ADD COLUMN {} {}", col_name, col_def);
                conn.execute(&sql, [])?;
            }
        }

        Ok(())
    }

    /// 迁移：更新 filename_pattern 旧默认值并修正 capture_region 值
    fn migrate_capture_region(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;

        // 所有非大地图区域统一为 0.626/0.648
        conn.execute(
            "UPDATE region_config SET capture_region_x = 0.626, capture_region_y = 0.648 WHERE name != '大地图'",
            [],
        )?;

        // 大地图区域特殊：0.378/0.388
        conn.execute(
            "UPDATE region_config SET capture_region_x = 0.378, capture_region_y = 0.388 WHERE name = '大地图'",
            [],
        )?;

        // 迁移 filename_pattern 旧默认值到新默认值
        // 仅当值仍为旧默认（即用户未自定义）时迁移，避免破坏用户自定义
        conn.execute(
            "UPDATE app_setting SET value = '{region}_{timestamp}_{scrollMode}' WHERE key = 'filename_pattern' AND value = '{prefix}_{timestamp}'",
            [],
        )?;

        Ok(())
    }

    /// 迁移：清理普通区域冗余 scroll_mode 记录（仅保留 0次）
    fn migrate_cleanup_redundant_modes(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;

        // 删除普通区域的非 0次 记录
        conn.execute(
            "DELETE FROM region_config
             WHERE scroll_mode != '0次'
             AND category NOT IN ('大地图', '自定义')",
            [],
        )?;

        Ok(())
    }

    /// 迁移：添加 target_w/target_h 列并反算填充
    fn migrate_add_target_columns(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;

        // 检查 target_w 列是否存在（幂等：已存在则跳过）
        let has_target_w: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('region_config') WHERE name = 'target_w'",
            [],
            |row| row.get::<_, i32>(0),
        )? > 0;

        if !has_target_w {
            conn.execute_batch(
                "ALTER TABLE region_config ADD COLUMN target_w INTEGER NOT NULL DEFAULT 0;
                 ALTER TABLE region_config ADD COLUMN target_h INTEGER NOT NULL DEFAULT 0;",
            )?;
        }

        // 对 target_w=0 的记录，从 grid/overlap 反算填充
        conn.execute(
            "UPDATE region_config
             SET target_w = CAST(
                    (CAST(1920 * capture_region_x AS INTEGER) -
                     CAST(CAST(1920 * capture_region_x AS INTEGER) * overlap_x AS INTEGER))
                    * grid_cols +
                    CAST(CAST(1920 * capture_region_x AS INTEGER) * overlap_x AS INTEGER) AS INTEGER
                 ),
                 target_h = CAST(
                    (CAST(1080 * capture_region_y AS INTEGER) -
                     CAST(CAST(1080 * capture_region_y AS INTEGER) * overlap_y AS INTEGER))
                    * grid_rows +
                    CAST(CAST(1080 * capture_region_y AS INTEGER) * overlap_y AS INTEGER) AS INTEGER
                 )
             WHERE target_w = 0 AND target_h = 0",
            [],
        )?;

        Ok(())
    }

    /// 首次启动写入默认数据
    fn seed_defaults(&self) -> AppResult<()> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = self.lock_conn()?;

        for (name, count, is_default) in SCROLL_MODES {
            conn.execute(
                "INSERT OR IGNORE INTO scroll_mode (name, scroll_count, is_default) VALUES (?, ?, ?)",
                params![name, count, is_default],
            )?;
        }

        Self::insert_default_regions(&conn, &now)?;

        for (key, value) in default_settings() {
            conn.execute(
                "INSERT OR IGNORE INTO app_setting (key, value, updated_at) VALUES (?, ?, ?)",
                params![key, value, now],
            )?;
        }

        conn.execute(
            "INSERT OR IGNORE INTO window_state (id, width, height, is_maximized, always_on_top, updated_at) VALUES (1, 900, 640, 0, 0, ?)",
            params![now],
        )?;

        Ok(())
    }

    /// 写入默认区域配置（仅插入 0次 记录）
    fn insert_default_regions(conn: &Connection, now: &str) -> AppResult<()> {
        for (name, category, scroll_mode, rows, cols, ox, oy, dx, dy, crx, cry, coy, tpl_ref, target_w, target_h) in
            default_regions()
        {
            if *name != *"大地图" && *name != *"自定义" && *scroll_mode != *"0次" {
                continue;
            }

            conn.execute(
                "INSERT OR IGNORE INTO region_config (name, category, aspect_ratio, scroll_mode, grid_rows, grid_cols, overlap_x, overlap_y, drag_x, drag_y, capture_region_x, capture_region_y, capture_offset_y, template_ref, target_w, target_h, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    name,
                    category,
                    "16:9",
                    scroll_mode,
                    rows,
                    cols,
                    ox,
                    oy,
                    dx,
                    dy,
                    crx,
                    cry,
                    coy,
                    tpl_ref,
                    target_w,
                    target_h,
                    now,
                    now
                ],
            )?;
        }

        Ok(())
    }

    // AppSetting

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.lock_conn()?;
        let result: Option<String> = conn.query_row(
            "SELECT value FROM app_setting WHERE key = ?",
            params![key],
            |row| row.get(0),
        ).optional()?;
        Ok(result)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO app_setting (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, now],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> AppResult<std::collections::HashMap<String, String>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT key, value FROM app_setting")?;
        let mut map = std::collections::HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (k, v) = row?;
            map.insert(k, v);
        }
        Ok(map)
    }

    // RegionConfig

    pub fn list_regions(&self) -> AppResult<Vec<RegionConfig>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("
            SELECT id, name, category, aspect_ratio, scroll_mode, grid_rows, grid_cols,
                   overlap_x, overlap_y, drag_x, drag_y,
                   capture_region_x, capture_region_y, capture_offset_y, template_ref,
                   target_w, target_h,
                   created_at, updated_at
            FROM region_config ORDER BY category, name, scroll_mode
        ")?;
        let rows = stmt.query_map([], row_to_region_config)?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    /// 按 name + aspect_ratio + scroll_mode 查询
    pub fn get_region(&self, name: &str, aspect_ratio: &str, scroll_mode: &str) -> AppResult<Option<RegionConfig>> {
        let conn = self.lock_conn()?;
        let result = conn.query_row(
            "SELECT id, name, category, aspect_ratio, scroll_mode, grid_rows, grid_cols,
                    overlap_x, overlap_y, drag_x, drag_y,
                    capture_region_x, capture_region_y, capture_offset_y, template_ref,
                    target_w, target_h,
                    created_at, updated_at
             FROM region_config WHERE name = ? AND aspect_ratio = ? AND scroll_mode = ?",
            params![name, aspect_ratio, scroll_mode],
            row_to_region_config,
        ).optional()?;
        Ok(result)
    }

    /// 从 0次记录实时推导指定次数的完整 RegionConfig
    ///
    /// 大地图/自定义：直接返回数据库值。0次：返回数据库值。
    /// 1-8次：用 0次 target + k + rate 推导，与 DataManageView 使用同一函数。
    pub fn derive_region_from_target(
        &self,
        region_name: &str,
        aspect_ratio: &str,
        scroll_mode: &str,
    ) -> AppResult<Option<RegionConfig>> {
        let region0 = match self.get_region(region_name, aspect_ratio, "0次")? {
            Some(r) => r,
            None => return Ok(None),
        };

        if region0.category == "大地图" || region0.category == "自定义" {
            if let Some(r) = self.get_region(region_name, aspect_ratio, scroll_mode)? {
                return Ok(Some(r));
            }
            return Ok(Some(region0));
        }

        let (target_w_0, target_h_0) = if region0.target_w > 0 && region0.target_h > 0 {
            (region0.target_w, region0.target_h)
        } else {
            let img_w = (1920_f64 * region0.capture_region_x).round() as i32;
            let img_h = (1080_f64 * region0.capture_region_y).round() as i32;
            let ovlp_px_x = (img_w as f64 * region0.overlap_x) as i32;
            let ovlp_px_y = (img_h as f64 * region0.overlap_y) as i32;
            let step_x = img_w - ovlp_px_x;
            let step_y = img_h - ovlp_px_y;
            (step_x * region0.grid_cols + ovlp_px_x, step_y * region0.grid_rows + ovlp_px_y)
        };

        if target_w_0 <= 0 || target_h_0 <= 0 {
            return Ok(Some(region0));
        }

        let overlap_min = self.get_setting("overlap_min")?
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let overlap_max = self.get_setting("overlap_max")?
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.5);

        let count = crate::models::scroll_mode::parse_scroll_count(&scroll_mode)
            .ok_or_else(|| AppError::new(
                format!("无效的 scroll_mode: {}", scroll_mode),
                "INVALID_SCROLL_MODE",
            ))?;
        let all_counts = crate::models::scroll_mode::derive_all_counts_from_base(
            1920, 1080, target_w_0, target_h_0, overlap_min, overlap_max,
        );
        let derived = match all_counts {
            Some(r) => match r.counts.get(count as usize) {
                Some(d) => *d,
                None => return Ok(Some(region0)),
            },
            None => return Ok(Some(region0)),
        };

        Ok(Some(RegionConfig {
            id: region0.id,
            name: region0.name.clone(),
            category: region0.category.clone(),
            aspect_ratio: region0.aspect_ratio.clone(),
            scroll_mode: scroll_mode.to_string(),
            grid_rows: derived.actual_rows,
            grid_cols: derived.actual_cols,
            overlap_x: derived.overlap_x,
            overlap_y: derived.overlap_y,
            drag_x: derived.drag_x,
            drag_y: derived.drag_y,
            capture_region_x: region0.capture_region_x,
            capture_region_y: region0.capture_region_y,
            capture_offset_y: region0.capture_offset_y,
            template_ref: region0.template_ref.clone(),
            target_w: target_w_0,
            target_h: target_h_0,
            created_at: region0.created_at.clone(),
            updated_at: region0.updated_at.clone(),
        }))
    }

    pub fn upsert_region(&self, config: &RegionConfig) -> AppResult<()> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO region_config (name, category, aspect_ratio, scroll_mode, grid_rows, grid_cols, overlap_x, overlap_y, drag_x, drag_y, capture_region_x, capture_region_y, capture_offset_y, template_ref, target_w, target_h, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(name, aspect_ratio, scroll_mode) DO UPDATE SET
                category = excluded.category,
                grid_rows = excluded.grid_rows,
                grid_cols = excluded.grid_cols,
                overlap_x = excluded.overlap_x,
                overlap_y = excluded.overlap_y,
                drag_x = excluded.drag_x,
                drag_y = excluded.drag_y,
                capture_region_x = excluded.capture_region_x,
                capture_region_y = excluded.capture_region_y,
                capture_offset_y = excluded.capture_offset_y,
                template_ref = excluded.template_ref,
                target_w = excluded.target_w,
                target_h = excluded.target_h,
                updated_at = excluded.updated_at",
            params![
                config.name, config.category, config.aspect_ratio, config.scroll_mode,
                config.grid_rows, config.grid_cols, config.overlap_x, config.overlap_y,
                config.drag_x, config.drag_y,
                config.capture_region_x, config.capture_region_y, config.capture_offset_y,
                config.template_ref, config.target_w, config.target_h, now, now
            ],
        )?;
        Ok(())
    }

    pub fn delete_region(&self, name: &str, aspect_ratio: &str, scroll_mode: &str) -> AppResult<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "DELETE FROM region_config WHERE name = ? AND aspect_ratio = ? AND scroll_mode = ?",
            params![name, aspect_ratio, scroll_mode],
        )?;
        Ok(())
    }

    // ScrollMode

    pub fn list_scroll_modes(&self) -> AppResult<Vec<ScrollMode>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("SELECT id, name, scroll_count, is_default FROM scroll_mode ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            Ok(ScrollMode {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                scroll_count: row.get(2)?,
                is_default: row.get(3)?,
            })
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    // WindowState

    pub fn load_window_state(&self) -> AppResult<WindowState> {
        let conn = self.lock_conn()?;
        let result = conn.query_row(
            "SELECT x, y, width, height, is_maximized, always_on_top, updated_at FROM window_state WHERE id = 1",
            [],
            |row| {
                Ok(WindowState {
                    x: row.get(0)?,
                    y: row.get(1)?,
                    width: row.get(2)?,
                    height: row.get(3)?,
                    is_maximized: row.get(4)?,
                    always_on_top: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        ).optional()?;
        Ok(result.unwrap_or_default())
    }

    pub fn save_window_state(&self, state: &WindowState) -> AppResult<()> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO window_state (id, x, y, width, height, is_maximized, always_on_top, updated_at)
             VALUES (1, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
                x = excluded.x, y = excluded.y, width = excluded.width, height = excluded.height,
                is_maximized = excluded.is_maximized, always_on_top = excluded.always_on_top,
                updated_at = excluded.updated_at",
            params![state.x, state.y, state.width, state.height, state.is_maximized, state.always_on_top, now],
        )?;
        Ok(())
    }

    // CaptureSession

    pub fn insert_session(&self, session: &CaptureSession) -> AppResult<i64> {
        let conn = self.lock_conn()?;
        conn.execute(
            "INSERT INTO capture_session (started_at, finished_at, region, scroll_mode, grid_rows, grid_cols, total_shots, status, original_path, exported_path, crop_box, output_format, jpg_quality)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                session.started_at, session.finished_at, session.region, session.scroll_mode,
                session.grid_rows, session.grid_cols, session.total_shots, session.status,
                session.original_path, session.exported_path, session.crop_box,
                session.output_format, session.jpg_quality
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn update_session(&self, id: i64, session: &CaptureSession) -> AppResult<()> {
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE capture_session SET finished_at = ?, status = ?, original_path = ?, exported_path = ?, crop_box = ?, output_format = ?, jpg_quality = ?, total_shots = ? WHERE id = ?",
            params![
                session.finished_at, session.status, session.original_path, session.exported_path,
                session.crop_box, session.output_format, session.jpg_quality, session.total_shots, id
            ],
        )?;
        Ok(())
    }

    /// 回写 session 的 original_path / exported_path / crop_box 字段
    /// None 表示不更新该字段
    pub fn update_session_paths(
        &self,
        session_id: i64,
        original_path: Option<&str>,
        exported_path: Option<&str>,
        crop_box: Option<&str>,
    ) -> AppResult<()> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let conn = self.lock_conn()?;
        conn.execute(
            "UPDATE capture_session SET
                original_path = COALESCE(?, original_path),
                exported_path = COALESCE(?, exported_path),
                crop_box = COALESCE(?, crop_box),
                finished_at = COALESCE(finished_at, ?)
             WHERE id = ?",
            params![original_path, exported_path, crop_box, now, session_id],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self, limit: u32) -> AppResult<Vec<CaptureSession>> {
        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare("
            SELECT id, started_at, finished_at, region, scroll_mode, grid_rows, grid_cols,
                   total_shots, status, original_path, exported_path, crop_box, output_format, jpg_quality
            FROM capture_session ORDER BY started_at DESC LIMIT ?
        ")?;
        let rows = stmt.query_map(params![limit], |row| {
            Ok(CaptureSession {
                id: Some(row.get(0)?),
                started_at: row.get(1)?,
                finished_at: row.get(2)?,
                region: row.get(3)?,
                scroll_mode: row.get(4)?,
                grid_rows: row.get(5)?,
                grid_cols: row.get(6)?,
                total_shots: row.get(7)?,
                status: row.get(8)?,
                original_path: row.get(9)?,
                exported_path: row.get(10)?,
                crop_box: row.get(11)?,
                output_format: row.get(12)?,
                jpg_quality: row.get(13)?,
            })
        })?;
        let mut list = Vec::new();
        for row in rows {
            list.push(row?);
        }
        Ok(list)
    }

    /// 清空所有历史会话记录
    pub fn clear_history(&self) -> AppResult<()> {
        let conn = self.lock_conn()?;
        conn.execute("DELETE FROM capture_session", params![])?;
        Ok(())
    }
}