//! History file I/O and basic entry storage.
//!
//! Uses SQLite for structured storage with indexed queries.
//! Falls back to TSV migration on first run.
//!
//! For searching/filtering, use `crate::history_search::HistorySearch`.
use crate::fuzzy::FuzzyVec;
use crate::theme::ThemeColor;
use rusqlite::{params, Connection, Result as SqlResult};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// A single history entry with full context.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// SQLite row id.
    pub id: i64,
    /// Unix timestamp when command was executed.
    pub timestamp: u64,
    /// Working directory when command was executed.
    pub cwd: PathBuf,
    /// Git branch (if in a git repo, None otherwise).
    pub git_branch: Option<String>,
    /// Exit status of the command.
    pub exit_status: i32,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// The command string.
    pub command: String,
}

/// Command history — SQLite storage and ordered entry cache.
pub struct History {
    path: PathBuf,
    conn: Connection,
    /// Fuzzy search index over commands (kept here for mainloop completion UI).
    command_index: FuzzyVec,
    /// cmd -> cwd mapping for completion cwd filter.
    path2cwd: HashMap<String, PathBuf>,
}

/// Open or create the SQLite history database.
fn open_db(db_path: &Path) -> SqlResult<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS history (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp   INTEGER NOT NULL,
            cwd         TEXT NOT NULL,
            git_branch  TEXT,
            exit_status INTEGER NOT NULL DEFAULT 0,
            duration_ms INTEGER NOT NULL DEFAULT 0,
            command     TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_command  ON history(command);
        CREATE INDEX IF NOT EXISTS idx_cwd      ON history(cwd);
        CREATE INDEX IF NOT EXISTS idx_branch   ON history(git_branch);
        CREATE INDEX IF NOT EXISTS idx_status   ON history(exit_status);
        CREATE INDEX IF NOT EXISTS idx_duration ON history(duration_ms);
        CREATE INDEX IF NOT EXISTS idx_ts       ON history(timestamp);",
    )?;
    Ok(conn)
}

/// Migrate a TSV history file into the SQLite database.
fn migrate_tsv(tsv_path: &Path, conn: &Connection) {
    if let Ok(file) = File::open(tsv_path) {
        let mut count = 0;
        for (i, line) in BufReader::new(file).lines().enumerate() {
            if let Ok(line) = line {
                let parts: Vec<&str> = line.split('\t').collect();

                if parts.len() >= 6 {
                    // New format: timestamp\tcwd\tbranch\texit_status\tduration_ms\tcommand
                    conn.execute(
                        "INSERT INTO history (timestamp, cwd, git_branch, exit_status, duration_ms, command)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            parts[0].parse::<i64>().unwrap_or(0),
                            parts[1],
                            if parts[2].is_empty() { None } else { Some(parts[2]) },
                            parts[3].parse::<i32>().unwrap_or(0),
                            parts[4].parse::<i64>().unwrap_or(0),
                            parts[5..].join("\t"),
                        ],
                    )
                    .ok();
                    count += 1;
                } else if parts.len() >= 3 {
                    // Old format: timestamp\tcwd\tcommand
                    conn.execute(
                        "INSERT INTO history (timestamp, cwd, git_branch, exit_status, duration_ms, command)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            parts[0].parse::<i64>().unwrap_or(0),
                            parts[1],
                            None::<&str>,
                            0,
                            0,
                            parts[2..].join("\t"),
                        ],
                    )
                    .ok();
                    count += 1;
                } else {
                    trace!("nsh: warning: failed to parse history at line {}", i + 1);
                }
            }
        }

        if count > 0 {
            info!("nsh: migrated {} entries from TSV to SQLite", count);
            // Rename old file so it doesn't get re-migrated
            let backup = tsv_path.with_extension("tsv.bak");
            fs::rename(tsv_path, &backup).ok();
        }
    }
}

impl History {
    pub fn new(history_file: &Path) -> History {
        // history_file is the old TSV path; we use a .db sibling
        // Use in-memory DB for tests (e.g. /dev/null)
        let (db_path, use_memory) = if history_file == Path::new("/dev/null")
            || history_file.starts_with("/tmp")
        {
            (PathBuf::from(":memory:"), true)
        } else {
            (history_file.with_extension("db"), false)
        };
        let tsv_exists = !use_memory && history_file.exists();

        let conn = if use_memory {
            match Connection::open_in_memory() {
                Ok(c) => c,
                Err(e) => panic!("nsh: failed to open in-memory history: {}", e),
            }
        } else {
            match open_db(&db_path) {
                Ok(c) => c,
                Err(e) => panic!("nsh: failed to open history database: {}", e),
            }
        };

        // Ensure schema exists (needed for in-memory DB)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp   INTEGER NOT NULL,
                cwd         TEXT NOT NULL,
                git_branch  TEXT,
                exit_status INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER NOT NULL DEFAULT 0,
                command     TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_command  ON history(command);
            CREATE INDEX IF NOT EXISTS idx_cwd      ON history(cwd);
            CREATE INDEX IF NOT EXISTS idx_branch   ON history(git_branch);
            CREATE INDEX IF NOT EXISTS idx_status   ON history(exit_status);
            CREATE INDEX IF NOT EXISTS idx_duration ON history(duration_ms);
            CREATE INDEX IF NOT EXISTS idx_ts       ON history(timestamp);",
        )
        .expect("failed to init history schema");

        // Migrate old TSV if it exists and DB is fresh
        if tsv_exists {
            let table_exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='history'",
                    [],
                    |row| row.get::<_, i32>(0).map(|c| c > 0),
                )
                .unwrap_or(false);

            // Only migrate if DB is empty (first run)
            if table_exists {
                let count: i64 = conn
                    .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
                    .unwrap_or(0);

                if count == 0 {
                    migrate_tsv(history_file, &conn);
                }
            }
        }

        // Build fuzzy index from database
        let mut command_index = FuzzyVec::new();
        let mut path2cwd: HashMap<String, PathBuf> = HashMap::new();

        {
            let mut stmt = conn
                .prepare("SELECT command, cwd FROM history ORDER BY id ASC")
                .expect("failed to prepare history query");

            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .expect("failed to query history");

            for row in rows {
                if let Ok((cmd, cwd)) = row {
                    command_index.append(cmd.clone());
                    path2cwd.insert(cmd, PathBuf::from(cwd));
                }
            }
        }

        History {
            path: db_path,
            conn,
            command_index,
            path2cwd,
        }
    }

    pub fn len(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM history", [], |row| row.get(0))
            .unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the nth most recent entry (0 = most recent).
    pub fn nth_last(&self, nth: usize) -> Option<String> {
        self.conn
            .query_row(
                "SELECT command FROM history ORDER BY id DESC LIMIT 1 OFFSET ?1",
                [nth],
                |row| row.get(0),
            )
            .ok()
    }

    /// Get the nth most recent entry with full context.
    pub fn nth_last_entry(&self, nth: usize) -> Option<HistoryEntry> {
        self.conn
            .query_row(
                "SELECT id, timestamp, cwd, git_branch, exit_status, duration_ms, command
                 FROM history ORDER BY id DESC LIMIT 1 OFFSET ?1",
                [nth],
                |row| {
                    Ok(HistoryEntry {
                        id: row.get(0)?,
                        timestamp: row.get::<_, i64>(1)? as u64,
                        cwd: PathBuf::from(row.get::<_, String>(2)?),
                        git_branch: row.get(3)?,
                        exit_status: row.get(4)?,
                        duration_ms: row.get::<_, i64>(5)? as u64,
                        command: row.get(6)?,
                    })
                },
            )
            .ok()
    }

    /// Fuzzy search index (for mainloop completion UI, cwd filter).
    /// For structured queries, use `HistorySearch`.
    pub fn search(&self, query: &str, filter_by_cwd: bool) -> Vec<(Option<ThemeColor>, &str)> {
        if filter_by_cwd {
            let cwd = std::env::current_dir().unwrap();
            self.command_index
                .search(query)
                .iter()
                .filter(|(_, cmd)| match self.path2cwd.get(*cmd) {
                    Some(path) if *path == cwd => true,
                    _ => false,
                })
                .cloned()
                .collect()
        } else {
            self.command_index.search(query)
        }
    }

    /// Execute a raw query on history, returning HistoryEntry rows.
    pub fn query_entries(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Vec<HistoryEntry> {
        let mut stmt = match self.conn.prepare(sql) {
            Ok(s) => s,
            Err(e) => {
                trace!("nsh: history query error: {}", e);
                return Vec::new();
            }
        };

        let rows = match stmt.query_map(params, |row| {
            Ok(HistoryEntry {
                id: row.get(0)?,
                timestamp: row.get::<_, i64>(1)? as u64,
                cwd: PathBuf::from(row.get::<_, String>(2)?),
                git_branch: row.get(3)?,
                exit_status: row.get(4)?,
                duration_ms: row.get::<_, i64>(5)? as u64,
                command: row.get(6)?,
            })
        }) {
            Ok(rows) => rows,
            Err(e) => {
                trace!("nsh: history query map error: {}", e);
                return Vec::new();
            }
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Execute a raw query, returning (theme, &str) tuples for fuzzy display.
    pub fn query_strings(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Vec<(Option<ThemeColor>, String)> {
        let mut stmt = match self.conn.prepare(sql) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let rows = match stmt.query_map(params, |row| row.get::<_, String>(0)) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };

        rows.filter_map(|r| r.ok().map(|cmd| (None, cmd)))
            .collect()
    }

    /// Append a command to history with full context.
    pub fn append(&mut self, cmd: &str, exit_status: i32, duration_ms: u64) {
        if cmd.is_empty() || cmd.len() < 8 {
            return;
        }

        // Ignore if `cmd` is same as the last command.
        if let Some(last) = self.nth_last(0) {
            if last == cmd {
                return;
            }
        }

        let cwd = std::env::current_dir().unwrap();
        let git_branch = get_git_branch();

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("failed to get the UNIX timestamp")
            .as_secs() as i64;

        self.conn
            .execute(
                "INSERT INTO history (timestamp, cwd, git_branch, exit_status, duration_ms, command)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    timestamp,
                    cwd.to_str().unwrap_or(""),
                    git_branch.as_deref(),
                    exit_status,
                    duration_ms as i64,
                    cmd,
                ],
            )
            .expect("failed to write history entry");

        // Update fuzzy index
        self.command_index.append(cmd.to_string());
        self.path2cwd.insert(cmd.to_string(), cwd);
    }

    /// Legacy append for backward compatibility.
    pub fn append_legacy(&mut self, cmd: &str) {
        self.append(cmd, 0, 0);
    }
}

/// Get the current git branch, if in a git repo.
pub(crate) fn get_git_branch() -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout);
        let branch = branch.trim().to_string();
        if !branch.is_empty() && branch != "HEAD" {
            return Some(branch);
        }
    }
    None
}

/// History selector for navigating through history.
pub struct HistorySelector {
    offset: usize,
    input: String,
}

impl HistorySelector {
    pub fn new() -> HistorySelector {
        HistorySelector {
            offset: 0,
            input: String::new(),
        }
    }

    pub fn reset(&mut self) {
        self.offset = 0;
    }

    /// Returns the current selection (or saved input if at offset 0).
    pub fn current(&self, history: &History) -> String {
        if self.offset == 0 {
            self.input.clone()
        } else {
            history.nth_last(self.offset - 1).unwrap()
        }
    }

    /// Select the previous (older) history entry.
    pub fn prev(&mut self, history: &History, input: &str) {
        if self.offset == 0 {
            self.input = input.to_string();
        }

        let hist_len = history.len();
        self.offset += 1;
        if self.offset >= hist_len {
            self.offset = hist_len;
        }
    }

    /// Select the next (newer) history entry.
    pub fn next(&mut self) {
        if self.offset > 0 {
            self.offset -= 1;
        }
    }
}
