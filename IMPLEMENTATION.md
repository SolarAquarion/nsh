# nsh Implementation Plan

## Phase 1: Foundation

### 1.1 Enhanced History ✅ DONE
- Commit: `e3c9abab` - Branch: `feature/enhanced-history`
- HistoryEntry struct with full context
- New format: `timestamp\tcwd\tbranch\texit_status\tduration_ms\tcommand`

### 1.2 Better Prompt ✅ DONE
- Commit: `3f781c6c` - Branch: `feature/better-prompt`
- New spans: last_status, duration, git_branch, git_status, time, date, load, env

### 1.3 SQLite History ✅ DONE
- Commit: `d04df1ba` - Branch: `feature/sqlite-history`
- Replaced TSV with SQLite for indexed queries
- Automatic migration from old format

### 1.4 Carapace Completions

**Goal:** Replace bash_server.rs with carapace as primary completer.

**Approach:**
1. Create `src/carapace_server.rs`
2. Spawn `carapace _carapace bash <command>` for completions
3. Parse JSON output into FuzzyVec
4. Keep bash_server.rs as fallback for unknown commands

**Changes:**
- `src/carapace_server.rs` — new module
- `src/mainloop.rs` — try carapace first, fall back to bash
- `src/completion.rs` — completion engine trait (interface for both)

**Completion Engine Trait:**
```rust
trait CompletionEngine {
    fn complete(&self, context: &CompletionContext) -> Vec<Completion>;
    fn name(&self) -> &str;
    fn priority(&self) -> u8;
}

struct CompletionContext {
    command: String,
    words: Vec<String>,
    current_word: usize,
    partial: String,
    cwd: PathBuf,
}

struct Completion {
    text: String,
    display: String,
    kind: CompletionKind,
}

enum CompletionKind {
    File,
    Directory,
    Command,
    Argument,
    Option,
    Value,
    Unknown,
}
```

**Testing:**
- Carapace returns completions for known commands
- Fallback to bash for unknown commands
- Completion UI works with both

---

### 1.5 Deep Path Completion

**Goal:** Murex-style deep completion. Type `nsh` → finds `~/nsh/src/main.rs`.

**Approach:**
1. Build a SQLite file index (background, lightweight)
2. Fuzzy match against index (not filesystem traversal)
3. Respect .gitignore by default
4. Alt+A to show all (including ignored files)

**File Index Schema:**
```sql
CREATE TABLE file_index (
    path TEXT PRIMARY KEY,
    name TEXT,
    dir TEXT,
    mtime INTEGER,
    atime INTEGER,
    depth INTEGER,
    ignored INTEGER DEFAULT 0
);
CREATE INDEX idx_name ON file_index(name);
CREATE INDEX idx_dir ON file_index(dir);
```

**Ranking:**
- Frecency: recent + frequent paths rank higher
- Depth: shallow paths rank higher
- Ignored: default hidden, Alt+A shows all

**Changes:**
- `src/file_index.rs` — new module (index, query, update)
- `src/completion.rs` — integrate deep path completion
- `src/mainloop.rs` — Alt+A keybinding for "show all"

---

## Phase 2: Interactive Experience

### 2.1 Editing Modes

**Goal:** Swappable editing layers, each complete.

**Architecture:**
```rust
trait EditMode {
    fn handle_key(&mut self, key: KeyEvent, buffer: &mut LineBuffer) -> EditAction;
    fn cursor_style(&self) -> CursorStyle;
    fn status_indicator(&self) -> Option<&str>;  // "-- NORMAL --" or None
    fn name(&self) -> &str;
}

enum EditAction {
    Insert(char),
    Delete(usize),
    Move(MoveDirection),
    Undo,
    Redo,
    Complete,
    Accept,
    Cancel,
    None,
}
```

**Modes:**
- `src/edit_readline.rs` — default (Ctrl+A/E/K/Y/W/U, Meta+B/F)
- `src/edit_vi.rs` — classical (hjkl, i/Esc, basic motions)
- `src/edit_vim.rs` — full (text objects, macros, registers, visual)
- `src/edit_emacs.rs` — C-x prefix, minibuffer, region
- `src/edit_helix.rs` — selection-first, multiple cursors

**Config:** `set edit-mode vi` in nshrc

---

### 2.2 Hint Text

**Goal:** Murex-style contextual help below prompt.

**Implementation:**
- Show command path + description from `whatis`
- Cached in SQLite for speed
- Update live as you type
- Show in bottom toolbar (like Murex)

**Changes:**
- `src/hint.rs` — new module (whatis lookup, caching, rendering)
- `src/mainloop.rs` — bottom toolbar integration

**Whatis Cache:**
```sql
CREATE TABLE whatis_cache (
    command TEXT PRIMARY KEY,
    path TEXT,
    description TEXT,
    updated INTEGER
);
```

---

### 2.3 Syntax Highlighting

**Goal:** Real-time highlighting as you type.

**Current:** nsh already has `highlight.rs`. Extend it.

**Token types:**
- Command (green)
- Flag/option (yellow)
- String (cyan)
- Path (blue)
- Variable (magenta)
- Error (red, underlined)
- Pipe (bold)
- Redirection (bold)

**Changes:**
- `src/highlight.rs` — extend with more token types
- `src/mainloop.rs` — render highlighted input

---

### 2.4 Autosuggestions

**Goal:** Fish-style inline suggestions from history.

**Implementation:**
- On each keystroke, search history for matching command
- Show as "ghost text" (dimmed, after cursor)
- Ctrl+F or → to accept
- Weighted by recency (recent commands rank higher)

**Changes:**
- `src/mainloop.rs` — ghost text rendering
- `src/history_search.rs` — prefix search for suggestions

---

### 2.5 Safer Pasting

**Goal:** Murex-style paste safety.

**Implementation:**
- Detect multi-line paste (bracketed paste mode)
- Show warning prompt: "You're about to execute N lines. [V]iew [E]xecute [C]ancel"
- Option to preview contents before executing

**Changes:**
- `src/mainloop.rs` — paste detection, warning UI

---

## Phase 3: Extension Bridge

### 3.1 JSON Protocol

**Goal:** Extensions communicate via JSON over stdin/stdout.

**Protocol:**
```json
// Request (nsh → extension)
{
  "type": "complete",
  "command": "kubectl",
  "words": ["kubectl", "get", "pods"],
  "current_word": 2,
  "partial": "",
  "cwd": "/home/user"
}

// Response (extension → nsh)
{
  "type": "completions",
  "completions": [
    {"text": "pod-1", "display": "pod-1 (Running)", "kind": "pod"},
    {"text": "pod-2", "display": "pod-2 (Pending)", "kind": "pod"}
  ]
}
```

**Changes:**
- `src/extension.rs` — new module (protocol, spawning, communication)

---

### 3.2 Extension Registry

**Config:** `~/.config/nsh/extensions.json`

```json
{
  "completions": {
    "carapace": {"command": "carapace _carapace bash", "priority": 1},
    "bash": {"command": "bash-completion", "priority": 2}
  },
  "hint": {
    "whatis": {"command": "whatis", "cache_ttl": 3600}
  }
}
```

---

### 3.3 Typed Pipes (Optional)

**Goal:** Murex-inspired dual-channel pipelines.

**Channels:**
- `|` — byte channel (text, traditional)
- `->` — value channel (structured data, when available)

**Implementation:**
- Internal value channel alongside stdout
- Commands can opt-in to structured output
- `put` command writes to value channel
- `echo` writes to byte channel
- Graceful fallback when structured isn't available

**Changes:**
- `src/eval.rs` — dual-channel pipeline execution
- `src/builtins/put.rs` — put command (writes to value channel)

---

## Phase 4: File Index

### 4.1 Background Indexer

**Goal:** Lightweight file index for deep completion.

**Implementation:**
- SQLite database at `~/.nsh/file_index.db`
- Updated on cd, ls, file operations (incremental)
- Full rebuild on first run or explicit request
- Background thread for updates (non-blocking)

**Changes:**
- `src/file_index.rs` — index, query, update, rebuild

---

### 4.2 Smart Filtering

**Goal:** Murex-style filtering with show-all option.

**Default:** skip .git/, target/, node_modules/, __pycache__/
**Alt+A:** show all results including ignored files
**`find:` prefix:** explicit search across everything

---

## Phase 5: Polish

### 5.1 Better Error Messages

```
nsh: command not found: gti
    Did you mean: git?

nsh: permission denied: /etc/shadow
    Try: sudo cat /etc/shadow
```

**Changes:**
- `src/eval.rs` — enhanced error handling
- Command suggestion (Levenshtein distance)

---

### 5.2 Command Preview

**Goal:** F9 previews command output while typing.

**Safe commands:** cat, grep, jq, wc, head, tail, sort, uniq
**Unsafe commands:** rm, mv, cp, sudo, chmod (require confirmation)

**Changes:**
- `src/mainloop.rs` — preview mode, safe command detection

---

### 5.3 Cross-Platform Polish

- macOS: PATH handling, homebrew integration
- Windows: WSL support
- FreeBSD/OpenBSD: job control, signals

---

## File Organization (Final)

```
src/
├── main.rs              # Entry point
├── mainloop.rs          # Event loop, TUI, input handling
├── shell.rs             # Shell state
├── eval.rs              # Command evaluation
├── parser.rs            # Shell grammar (PEG)
├── history.rs           # History storage (SQLite)
├── history_search.rs    # History search and filtering
├── prompt.rs            # Prompt rendering and spans
├── completion.rs        # Completion engine trait
├── carapace_server.rs   # Carapace completion
├── bash_server.rs       # Bash completion fallback
├── expand.rs            # Variable/command expansion
├── highlight.rs         # Syntax highlighting
├── process.rs           # Process management
├── file_index.rs        # Background file indexer
├── edit_mode.rs         # Editing mode trait
├── edit_readline.rs     # Readline editing (default)
├── edit_vi.rs           # Vi editing (classical)
├── edit_vim.rs          # Vim editing (full)
├── edit_emacs.rs        # Emacs editing
├── edit_helix.rs        # Helix editing
├── extension.rs         # Extension bridge protocol
├── hint.rs              # Hint text (whatis lookup)
└── builtins/            # Built-in commands
```
