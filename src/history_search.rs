//! History search and filtering.
//!
//! Provides rich querying over history entries: by branch, exit status,
//! duration, and combined filters. Depends on `crate::history::History`/`HistoryEntry`.
use crate::history::History;
use crate::history::HistoryEntry;
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

/// Search and filter history entries.
///
/// Takes a `&History` reference and applies various filters.
pub struct HistorySearch<'a> {
    history: &'a History,
}

impl<'a> HistorySearch<'a> {
    pub fn new(history: &'a History) -> Self {
        HistorySearch { history }
    }

    /// Search history by command (fuzzy), with optional cwd filter.
    pub fn search(&self, query: &str, filter_by_cwd: bool) -> Vec<(Option<ThemeColor>, &'a str)> {
        self.history.search(query, filter_by_cwd)
    }

    /// All entries that ran in a specific git branch.
    pub fn by_branch(&self, branch: &str) -> Vec<&'a HistoryEntry> {
        self.history
            .entries()
            .iter()
            .filter(|e| e.git_branch.as_deref() == Some(branch))
            .collect()
    }

    /// All entries with non-zero exit status.
    pub fn failed(&self) -> Vec<&'a HistoryEntry> {
        self.history
            .entries()
            .iter()
            .filter(|e| e.exit_status != 0)
            .collect()
    }

    /// All entries with duration >= threshold.
    pub fn slow(&self, threshold_ms: u64) -> Vec<&'a HistoryEntry> {
        self.history
            .entries()
            .iter()
            .filter(|e| e.duration_ms >= threshold_ms)
            .collect()
    }

    /// Entries matching multiple filters simultaneously.
    pub fn advanced(
        &self,
        query: &str,
        filter: &HistoryFilter,
    ) -> Vec<(Option<ThemeColor>, &'a str)> {
        let cwd = std::env::current_dir().ok();

        self.history
            .search(query, false)
            .into_iter()
            .filter(|(_, cmd)| {
                self.history
                    .entries()
                    .iter()
                    .rev() // most recent first
                    .find(|e| e.command == *cmd)
                    .map(|entry| {
                        if filter.cwd_only {
                            if let Some(ref cwd) = cwd {
                                if entry.cwd != *cwd {
                                    return false;
                                }
                            }
                        }

                        if let Some(ref branch) = filter.branch {
                            if entry.git_branch.as_ref() != Some(branch) {
                                return false;
                            }
                        }

                        if filter.failed_only && entry.exit_status == 0 {
                            return false;
                        }

                        if let Some(min_duration) = filter.min_duration_ms {
                            if entry.duration_ms < min_duration {
                                return false;
                            }
                        }

                        true
                    })
                    .unwrap_or(false)
            })
            .collect()
    }
}
