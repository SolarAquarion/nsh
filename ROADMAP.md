# nsh Fork Roadmap

> A modern POSIX-compatible shell with AI integration, structured data, and IDE-like completions.

## Vision

Build on nsh's solid foundation to create a shell that:
1. **Remains POSIX compatible** — existing scripts Just Work™
2. **Adds modern features** — structured data, AI, context awareness
3. **Prioritizes UX** — zero-config, fast, intuitive

## Phase 1: Polish (Low Risk)

### 1.1 Enhanced History
**File:** `src/history.rs`

Current format: `timestamp\tcwd\tcommand\n`

Extended format: `timestamp\tcwd\tgit_branch\texit_status\tduration_ms\tcommand\n`

```rust
struct HistoryEntry {
    timestamp: u64,
    cwd: PathBuf,
    git_branch: Option<String>,
    exit_status: i32,
    duration_ms: u64,
    command: String,
}
```

**Benefits:**
- Search history by git branch
- Filter by exit status (failed commands)
- Analyze command duration patterns

**Difficulty:** Easy (local changes only)

---

### 1.2 Better Prompt
**File:** `src/prompt.rs`, `src/prompt.pest`

Add prompt functions:
- `\{last_status}` — exit status of last command
- `\{duration}` — duration of last command
- `\{git_branch}` — current git branch (extract from repo_status)
- `\{git_status}` — clean/dirty indicator
- `\{time}` — current time
- `\{load}` — system load average
- `\{env:VAR}` — environment variable

Custom prompt functions via config:
```rust
// In ~/.config/nsh/nshrc
prompt_function "kube" { kubectl config current-context }
```

**Difficulty:** Easy

---

### 1.3 Native Completion Engine
**File:** Replace `src/bash_server.rs`

Current: Spawns bash subprocess for completions

Goal: Native Rust completion engine

Approach:
1. Parse completion specs (similar to bash `complete`)
2. Support file, directory, command, host completions natively
3. Allow external completion providers (via stdin/stdout protocol)
4. Keep bash completion as fallback

```rust
struct CompletionSpec {
    command: String,
    completions: CompletionType,
}

enum CompletionType {
    Files { pattern: Option<String> },
    Directory,
    Commands,
    Hosts,
    Users,
    Custom { command: String },  // external provider
}
```

**Difficulty:** Medium

---

## Phase 2: Innovation (Medium Risk)

### 2.1 AI Integration
**New File:** `src/ai.rs`

Features:
- **Inline suggestions** — ghost text for likely next command
- **Command explanation** — `?? <command>` explains what it does
- **Error diagnosis** — when command fails, suggest fix
- **Natural language** — `nsh: find large files over 100MB`

```rust
struct AIConfig {
    enabled: bool,
    provider: AIProvider,  // openai, anthropic, local
    api_key: Option<String>,
    model: String,
    max_suggestions: usize,
}

impl AI {
    fn suggest_next(&self, context: &ShellContext) -> Option<String>;
    fn explain(&self, command: &str) -> String;
    fn diagnose_error(&self, cmd: &str, error: &str) -> Option<String>;
    fn natural_language(&self, query: &str) -> Option<String>;
}
```

**Difficulty:** Medium (API integration is straightforward, UX is the challenge)

---

### 2.2 Structured Data (Opt-in)
**New File:** `src/structured.rs`

Approach: Commands can opt-in to structured output via special variable or prefix.

```sh
# Structured output mode
$ export NSH_STRUCTURED=1
$ ls --json | where size > 1MB | sort-by modified
```

Or via pipe operator:
```sh
$ ls |> structured |> where size > 1MB
```

Internal representation:
```rust
enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Null,
}

struct Table {
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
}
```

**Difficulty:** Hard (requires type system, parsing, and careful POSIX compatibility)

---

### 2.3 Plugin System
**New File:** `src/plugin.rs`

Approach: WebAssembly-based plugins for safety and portability.

```rust
trait Plugin {
    fn name(&self) -> &str;
    fn completions(&self, context: &CompletionContext) -> Vec<String>;
    fn hooks(&self) -> Vec<Hook>;
}

enum Hook {
    BeforeCommand(Box<dyn Fn(&str)>),
    AfterCommand(Box<dyn Fn(&str, i32)>),
    Prompt(Box<dyn Fn() -> String>),
}
```

Plugin directory: `~/.config/nsh/plugins/`

**Difficulty:** Medium

---

## Phase 3: Revolution (High Risk)

### 3.1 IDE-like Completions

- **Type-aware completions** — know that `git checkout` expects branch names
- **Context-aware suggestions** — suggest based on directory contents
- **Semantic highlighting** — different colors for different token types
- **Inline documentation** — show help in completion menu

**Difficulty:** Hard

---

### 3.2 Semantic Understanding

- Parse command output into structured data when possible
- Remember command relationships (`make` → `Makefile`)
- Suggest next commands based on workflow patterns

**Difficulty:** Very Hard

---

## Configuration

Config file: `~/.config/nsh/nshrc`

```sh
# Example nshrc
setopt auto_pushd
setopt pushd_ignore_dups

# Prompt
PROMPT='\{cyan}\{bold}\{username}@\{hostname}\{reset}:\{current_dir}\{git_status} $ '

# AI integration
ai enable
ai provider openai
ai model gpt-4

# Plugins
plugin load git-enhancements
plugin load docker-completions
```

---

## Architecture Extensions

```
src/
├── ai.rs              # NEW: AI integration
├── structured.rs      # NEW: Structured data types
├── plugin.rs          # NEW: Plugin system
├── completion.rs      # NEW: Native completion engine
├── config.rs          # NEW: Config file parsing
├── history.rs         # EXTENDED: Enhanced history
├── prompt.rs          # EXTENDED: More prompt functions
├── mainloop.rs        # EXTENDED: AI suggestions UI
└── ...existing...
```

---

## Contributing Back

Features that don't break POSIX compatibility will be upstreamed to nuta/nsh:
- Enhanced history (configurable format)
- Better prompt functions
- Native completion engine

---

## Milestones

- [x] **v0.5.0** — Enhanced history (git branch, exit status, duration) ✅ DONE
- [x] **v0.5.1** — Better prompt (new spans: last_status, duration, git_branch, git_status) ✅ DONE
- [x] **v0.5.2** — SQLite-backed history with indexed queries ✅ DONE
- [ ] **v0.6.0** — Native completion engine
- [ ] **v0.7.0** — AI integration
- [ ] **v0.8.0** — Structured data (opt-in)
- [ ] **v1.0.0** — Plugin system, stable API
