//! History file I/O and basic entry storage.
//!
//! This module handles reading/writing history from disk and provides
//! basic access (nth, len, append). For searching and filtering,
//! use `crate::history_search::HistorySearch`.
use crate::fuzzy::FuzzyVec;
use crate::theme::ThemeColor;
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// A single history entry with full context.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
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

/// Command history — file I/O and ordered entry storage.
pub struct History {
    path: PathBuf,
    /// Ordered list of history entries.
    entries: Vec<HistoryEntry>,
    /// Fuzzy search index over commands (kept here for mainloop completion UI).
    command_index: FuzzyVec,
    /// Compatibility: old format used cmd -> cwd mapping.
    path2cwd: HashMap<String, PathBuf>,
}

impl History {
    pub fn new(history_file: &Path) -> History {
        let mut entries = Vec::new();
        let mut command_index = FuzzyVec::new();
        let mut path2cwd: HashMap<String, PathBuf> = HashMap::new();

        if let Ok(file) = File::open(history_file) {
            for (i, line) in BufReader::new(file).lines().enumerate() {
                if let Ok(line) = line {
                    let parts: Vec<&str> = line.split('\t').collect();

                    let entry = if parts.len() >= 6 {
                        // New format: timestamp\tcwd\tbranch\texit_status\tduration_ms\tcommand
                        HistoryEntry {
                            timestamp: parts[0].parse().unwrap_or(0),
                            cwd: PathBuf::from(parts[1]),
                            git_branch: if parts[2].is_empty() {
                                None
                            } else {
                                Some(parts[2].to_string())
                            },
                            exit_status: parts[3].parse().unwrap_or(0),
                            duration_ms: parts[4].parse().unwrap_or(0),
                            command: parts[5..].join("\t"), // Command may contain tabs
                        }
                    } else if parts.len() >= 3 {
                        // Old format: timestamp\tcwd\tcommand
                        HistoryEntry {
                            timestamp: parts[0].parse().unwrap_or(0),
                            cwd: PathBuf::from(parts[1]),
                            git_branch: None,
                            exit_status: 0,
                            duration_ms: 0,
                            command: parts[2..].join("\t"),
                        }
                    } else {
                        // Malformed line
                        trace!("nsh: warning: failed to parse history at line {}", i + 1);
                        continue;
                    };

                    command_index.append(entry.command.clone());
                    path2cwd.insert(entry.command.clone(), entry.cwd.clone());
                    entries.push(entry);
                }
            }
        }

        History {
            path: history_file.to_owned(),
            entries,
            command_index,
            path2cwd,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Iterate all entries.
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Get the nth most recent entry (0 = most recent).
    pub fn nth_last(&self, nth: usize) -> Option<String> {
        if nth >= self.entries.len() {
            return None;
        }
        let idx = self.entries.len() - 1 - nth;
        self.entries.get(idx).map(|e| e.command.clone())
    }

    /// Get the nth most recent entry with full context.
    pub fn nth_last_entry(&self, nth: usize) -> Option<&HistoryEntry> {
        if nth >= self.entries.len() {
            return None;
        }
        let idx = self.entries.len() - 1 - nth;
        self.entries.get(idx)
    }

    /// Fuzzy search index (for mainloop completion UI, cwd filter).
    /// Prefer `HistorySearch` for rich filtering.
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

    /// Append a command to history with full context.
    pub fn append(&mut self, cmd: &str, exit_status: i32, duration_ms: u64) {
        if cmd.is_empty() || cmd.len() < 8 {
            return;
        }

        // Ignore if `cmd` is same as the last command.
        if let Some(last) = self.entries.last() {
            if last.command == cmd {
                return;
            }
        }

        let cwd = std::env::current_dir().unwrap();
        let git_branch = get_git_branch();

        let entry = HistoryEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("failed to get the UNIX timestamp")
                .as_secs(),
            cwd: cwd.clone(),
            git_branch: git_branch.clone(),
            exit_status,
            duration_ms,
            command: cmd.to_string(),
        };

        // Write to file
        if let Ok(mut file) = OpenOptions::new().append(true).open(&self.path) {
            let branch_str = git_branch.as_deref().unwrap_or("");
            let line = format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                entry.timestamp,
                entry.cwd.to_str().unwrap_or(""),
                branch_str,
                entry.exit_status,
                entry.duration_ms,
                entry.command
            );
            file.write(line.as_bytes()).ok();
        }

        // Update indices
        self.command_index.append(cmd.to_string());
        self.path2cwd.insert(cmd.to_string(), cwd);
        self.entries.push(entry);
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
