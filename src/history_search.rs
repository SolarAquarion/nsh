//! History search and filtering via SQL queries.
//!
//! Provides rich querying over history entries: by branch, exit status,
//! duration, and combined filters. Uses the SQLite backend directly.
use crate::history::{History, HistoryEntry};
use crate::theme::ThemeColor;

/// Filter options for history queries.
#[derive(Debug, Clone, Default)]
pub struct HistoryFilter {
    /// Only show commands from current directory.
    pub cwd_only: bool,
    /// Filter by git branch.
    pub branch: Option<String>,
    /// Only show failed commands.
    pub failed_only: bool,
    /// Minimum duration in milliseconds.
    pub min_duration_ms: Option<u64>,
}

/// Search and filter history entries via SQL.
///
/// Takes a `&History` reference and builds SQL queries for fast lookup.
pub struct HistorySearch<'a> {
    history: &'a History,
}

impl<'a> HistorySearch<'a> {
    pub fn new(history: &'a History) -> Self {
        HistorySearch { history }
    }

    /// Fuzzy search (delegates to FuzzyVec index for compatibility).
    pub fn search(&self, query: &str, filter_by_cwd: bool) -> Vec<(Option<ThemeColor>, &'a str)> {
        self.history.search(query, filter_by_cwd)
    }

    /// All entries that ran in a specific git branch.
    pub fn by_branch(&self, branch: &str) -> Vec<HistoryEntry> {
        self.history.query_entries(
            "SELECT id, timestamp, cwd, git_branch, exit_status, duration_ms, command
             FROM history WHERE git_branch = ?1 ORDER BY id DESC",
            &[&branch],
        )
    }

    /// All entries with non-zero exit status.
    pub fn failed(&self) -> Vec<HistoryEntry> {
        self.history.query_entries(
            "SELECT id, timestamp, cwd, git_branch, exit_status, duration_ms, command
             FROM history WHERE exit_status != 0 ORDER BY id DESC",
            &[],
        )
    }

    /// All entries with duration >= threshold.
    pub fn slow(&self, threshold_ms: u64) -> Vec<HistoryEntry> {
        self.history.query_entries(
            "SELECT id, timestamp, cwd, git_branch, exit_status, duration_ms, command
             FROM history WHERE duration_ms >= ?1 ORDER BY duration_ms DESC",
            &[&(threshold_ms as i64)],
        )
    }

    /// Entries matching multiple filters simultaneously.
    pub fn advanced(
        &self,
        query: &str,
        filter: &HistoryFilter,
    ) -> Vec<(Option<ThemeColor>, String)> {
        let mut sql = String::from(
            "SELECT DISTINCT command FROM history WHERE 1=1"
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Fuzzy match on command text
        if !query.is_empty() {
            sql.push_str(" AND command LIKE ?");
            params.push(Box::new(format!("%{}%", query)));
        }

        if filter.cwd_only {
            if let Ok(cwd) = std::env::current_dir() {
                sql.push_str(" AND cwd = ?");
                params.push(Box::new(cwd.to_str().unwrap_or("").to_string()));
            }
        }

        if let Some(ref branch) = filter.branch {
            sql.push_str(" AND git_branch = ?");
            params.push(Box::new(branch.clone()));
        }

        if filter.failed_only {
            sql.push_str(" AND exit_status != 0");
        }

        if let Some(min_duration) = filter.min_duration_ms {
            sql.push_str(" AND duration_ms >= ?");
            params.push(Box::new(min_duration as i64));
        }

        sql.push_str(" ORDER BY id DESC LIMIT 100");

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p.as_ref()).collect();

        self.history.query_strings(&sql, &param_refs)
    }
}
