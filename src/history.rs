//! History management with enhanced context tracking.
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

/// Command history with enhanced context tracking.
pub struct History {
    path: PathBuf,
    /// Ordered list of history entries.
    entries: Vec<HistoryEntry>,
    /// Fuzzy search index over commands.
    command_index: FuzzyVec,
    /// Index from command to entry indices (for fast lookup).
    cmd_to_entries: HashMap<String, Vec<usize>>,
    /// Index from cwd to entry indices.
    cwd_to_entries: HashMap<PathBuf, Vec<usize>>,
    /// Index from git branch to entry indices.
    branch_to_entries: HashMap<String, Vec<usize>>,
    /// Compatibility: old format used cmd -> cwd mapping.
    path2cwd: HashMap<String, PathBuf>,
}

impl History {
    pub fn new(history_file: &Path) -> History {
        let mut entries = Vec::new();
        let mut command_index = FuzzyVec::new();
        let mut cmd_to_entries: HashMap<String, Vec<usize>> = HashMap::new();
        let mut cwd_to_entries: HashMap<PathBuf, Vec<usize>> = HashMap::new();
        let mut branch_to_entries: HashMap<String, Vec<usize>> = HashMap::new();
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
                    
                    let idx = entries.len();
                    entries.push(entry.clone());
                    command_index.append(entry.command.clone());
                    
                    cmd_to_entries
                        .entry(entry.command.clone())
                        .or_default()
                        .push(idx);
                    
                    cwd_to_entries
                        .entry(entry.cwd.clone())
                        .or_default()
                        .push(idx);
                    
                    if let Some(ref branch) = entry.git_branch {
                        branch_to_entries
                            .entry(branch.clone())
                            .or_default()
                            .push(idx);
                    }
                    
                    // Compatibility with old API
                    path2cwd.insert(entry.command.clone(), entry.cwd.clone());
                }
            }
        }

        History {
            path: history_file.to_owned(),
            entries,
            command_index,
            cmd_to_entries,
            cwd_to_entries,
            branch_to_entries,
            path2cwd,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
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

    /// Search history by command (fuzzy).
    pub fn search(&self, query: &str, filter_by_cwd: bool) -> Vec<(Option<ThemeColor>, &str)> {
        if filter_by_cwd {
            let cwd = std::env::current_dir().unwrap();
            self.command_index
                .search(query)
                .iter()
                .filter(|(_, cmd)| match self.path2cwd.get(*cmd) {
                    Some(path) if *path == cwd => true,
                    Some(path) => {
                        info!("path='{}' {}", path.display(), cwd.display());
                        false
                    }
                    _ => false,
                })
                .cloned()
                .collect()
        } else {
            self.command_index.search(query)
        }
    }

    /// Search history by git branch.
    pub fn search_by_branch(&self, branch: &str) -> Vec<&HistoryEntry> {
        self.branch_to_entries
            .get(branch)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&idx| self.entries.get(idx))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Search for failed commands (non-zero exit status).
    pub fn search_failed(&self) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.exit_status != 0)
            .collect()
    }

    /// Search for slow commands (duration >= threshold_ms).
    pub fn search_slow(&self, threshold_ms: u64) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .filter(|e| e.duration_ms >= threshold_ms)
            .collect()
    }

    /// Search history with multiple filters.
    pub fn search_advanced(
        &self,
        query: &str,
        filter: HistoryFilter,
    ) -> Vec<(Option<ThemeColor>, &str)> {
        let cwd = std::env::current_dir().ok();
        
        self.command_index
            .search(query)
            .iter()
            .filter(|(_, cmd)| {
                // Get the most recent entry for this command
                if let Some(indices) = self.cmd_to_entries.get(*cmd) {
                    if let Some(&idx) = indices.last() {
                        if let Some(entry) = self.entries.get(idx) {
                            // Apply filters
                            if filter.cwd_only {
                                if let Some(ref cwd) = cwd {
                                    if entry.cwd != *cwd {
                                        return false;
                                    }
                                }
                            }
                            
                            if let Some(branch) = &filter.branch {
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
                            
                            return true;
                        }
                    }
                }
                false
            })
            .cloned()
            .collect()
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
        let idx = self.entries.len();
        self.entries.push(entry.clone());
        self.command_index.append(cmd.to_string());
        
        self.cmd_to_entries
            .entry(cmd.to_string())
            .or_default()
            .push(idx);
        
        self.cwd_to_entries
            .entry(cwd.clone())
            .or_default()
            .push(idx);
        
        if let Some(branch) = git_branch {
            self.branch_to_entries
                .entry(branch)
                .or_default()
                .push(idx);
        }
        
        // Compatibility with old API
        self.path2cwd.insert(cmd.to_string(), cwd);
    }

    /// Legacy append for backward compatibility.
    pub fn append_legacy(&mut self, cmd: &str) {
        self.append(cmd, 0, 0);
    }
}

/// Filter options for advanced history search.
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

/// Get the current git branch, if in a git repo.
fn get_git_branch() -> Option<String> {
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
