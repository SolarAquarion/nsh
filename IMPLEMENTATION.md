# Implementation Plan: Phase 1

## 1. Enhanced History

### Current State
```rust
// history.rs
pub struct History {
    path: PathBuf,
    history: FuzzyVec,
    path2cwd: HashMap<String, PathBuf>,  // cmd → cwd
}

// File format: timestamp\tcwd\tcommand\n
```

### New State
```rust
// history.rs
pub struct HistoryEntry {
    pub timestamp: u64,
    pub cwd: PathBuf,
    pub git_branch: Option<String>,
    pub exit_status: i32,
    pub duration_ms: u64,
    pub command: String,
}

pub struct History {
    path: PathBuf,
    entries: Vec<HistoryEntry>,
    by_command: FuzzyIndex,    // for fuzzy search
    by_cwd: HashMap<PathBuf, Vec<usize>>,
    by_branch: HashMap<String, Vec<usize>>,
}

// File format: timestamp\tcwd\tbranch\texit_status\tduration_ms\tcommand\n
// Branch is empty string if not in git repo
```

### Implementation Steps

1. **Add `HistoryEntry` struct**
   - Create new struct with all fields
   - Add serialization/deserialization

2. **Update `History::append()`**
   - Get git branch: run `git rev-parse --abbrev-ref HEAD`
   - Get exit status: passed from mainloop
   - Get duration: track command start/end time

3. **Update `History::new()`**
   - Parse new format
   - Maintain backward compatibility with old format

4. **Add search methods**
   - `search_by_branch(branch: &str) -> Vec<&HistoryEntry>`
   - `search_failed() -> Vec<&HistoryEntry>` (exit_status != 0)
   - `search_slow(threshold_ms: u64) -> Vec<&HistoryEntry>`

5. **Update mainloop.rs**
   - Track command start time
   - Pass exit status and duration to `history.append()`

### Testing
- Old history files load correctly
- New entries save with all fields
- Search by branch/exit status works

---

## 2. Better Prompt

### Current State
```rust
// prompt.rs
enum Span {
    Literal(String),
    Color(Color),
    Username,
    Hostname,
    CurrentDir,
    Newline,
    RepoStatus,
    If { condition, then_part, else_part },
}
```

### New Spans to Add
```rust
enum Span {
    // ...existing...
    
    // New spans
    LastStatus,      // Exit status of last command
    LastDuration,    // Duration of last command
    Time,            // Current time (HH:MM:SS)
    Date,            // Current date (YYYY-MM-DD)
    Load,            // System load average
    GitBranch,       // Current git branch (from repo_status)
    GitStatus,       // Clean/dirty indicator (* if modified)
    EnvVar(String),  // Environment variable
    
    // Custom function
    CustomFunction { name: String, args: Vec<String> },
}
```

### Implementation Steps

1. **Add new span types to `prompt.pest`**
   ```
   last_status_span    = { "\\{last_status}" }
   last_duration_span  = { "\\{last_duration}" }
   time_span           = { "\\{time}" }
   date_span           = { "\\{date}" }
   load_span           = { "\\{load}" }
   git_branch_span     = { "\\{git_branch}" }
   git_status_span     = { "\\{git_status}" }
   env_var_span        = { "\\{env:" ~ var_name ~ "}" }
   ```

2. **Update `draw_prompt()`**
   - Store last status and duration in Shell struct
   - Get system load from `/proc/loadavg` (Linux) or `sysctl` (macOS)
   - Extract git info from existing `get_repo_info()`

3. **Add custom functions via config**
   - In `~/.config/nsh/nshrc`:
     ```sh
     prompt_func kube "kubectl config current-context 2>/dev/null"
     ```
   - Use as `\{func:kube}` in prompt

### Testing
- All new spans render correctly
- Custom functions execute safely
- Performance is acceptable (< 10ms prompt render)

---

## 3. Native Completion Engine

### Current State
```rust
// bash_server.rs
// Spawns bash subprocess, sends completion requests
// Returns FuzzyVec of completions
```

### New Architecture
```rust
// completion.rs (new file)

pub struct CompletionEngine {
    specs: HashMap<String, CompletionSpec>,
    providers: Vec<Box<dyn CompletionProvider>>,
}

pub enum CompletionSpec {
    Default,  // files + commands
    Files { pattern: Option<GlobPattern> },
    Directories,
    Commands,
    Hosts,
    Users,
    Groups,
    Services,
    Environment,
    Alias,
    Custom { command: String, args: Vec<String> },
}

pub trait CompletionProvider {
    fn name(&self) -> &str;
    fn provides(&self) -> &[&str];  // commands this provider handles
    fn complete(&self, context: &CompletionContext) -> Vec<Completion>;
}

pub struct CompletionContext {
    pub command: String,
    pub words: Vec<String>,
    pub current_word: usize,
    pub current_word_partial: String,
    pub shell: &Shell,
}

pub struct Completion {
    pub text: String,
    pub display: String,      // for display (may include description)
    pub kind: CompletionKind,
}

pub enum CompletionKind {
    File,
    Directory,
    Command,
    Argument,
    Option,
    Value,
}
```

### Built-in Providers

1. **FileProvider** — file/directory completions
2. **CommandProvider** — commands from PATH + builtins
3. **HostProvider** — from `/etc/hosts` + SSH known_hosts
4. **EnvProvider** — environment variable names
5. **AliasProvider** — shell aliases

### External Providers

Protocol: JSON over stdin/stdout

```json
// Request (to provider)
{
  "command": "kubectl",
  "words": ["kubectl", "get", "pods", "-n", ""],
  "current_word": 4,
  "current_word_partial": ""
}

// Response (from provider)
{
  "completions": [
    {"text": "default", "display": "default (current namespace)", "kind": "value"},
    {"text": "kube-system", "display": "kube-system", "kind": "value"}
  ]
}
```

### Configuration
```sh
# ~/.config/nsh/completions/kubectl.sh
complete -C "kubectl __complete" kubectl
```

### Implementation Steps

1. **Create `completion.rs`**
   - Define `CompletionEngine`, `CompletionSpec`, traits

2. **Implement built-in providers**
   - FileProvider (use existing `path_completion()`)
   - CommandProvider (use existing `PathTable`)
   - HostProvider
   - EnvProvider

3. **Add completion specs**
   - Parse from config file
   - Default specs for common commands

4. **Update `mainloop.rs`**
   - Call `CompletionEngine::complete()` instead of bash_server
   - Keep bash_server as fallback for unknown commands

5. **Add external provider protocol**
   - Spawn provider process
   - JSON communication

### Testing
- File completion works
- Command completion works
- External providers work
- Fallback to bash works

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src/history.rs` | Add `HistoryEntry`, enhanced fields, search methods |
| `src/shell.rs` | Add `last_status`, `last_duration` fields |
| `src/mainloop.rs` | Track timing, pass to history |
| `src/prompt.rs` | Add new span types |
| `src/prompt.pest` | Add new grammar rules |
| `src/completion.rs` | NEW: Native completion engine |
| `src/bash_server.rs` | Refactor to fallback provider |

---

## Timeline

- **Week 1:** Enhanced history
- **Week 2:** Better prompt
- **Week 3-4:** Native completion engine
