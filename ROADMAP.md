# nsh Roadmap

> The best default shell.

## Vision

nsh is a **platform**, not a language. The shell provides the interactive experience (REPL, TUI, completions, history). The scripting language lives outside.

**Core principles:**
1. **Great defaults** — install and immediately enjoy, no config needed
2. **Optional replaceability** — every layer is swappable
3. **Extending power** — call any tool, any language
4. **Language-agnostic** — extensions don't depend on nsh's language
5. **Cross-platform** — single Rust binary, runs on any OS

**What nsh is NOT:**
- Not a scripting language (delegate to bash, Python, Clojure)
- Not POSIX-compatible (use bash when you need POSIX)
- Not a plugin ecosystem (extensions are external processes)

## Architecture

```
nsh (Rust binary, zero dependencies)
    │
    ├── Built-in language (basic: commands, pipes, variables, functions)
    │
    ├── Extension bridge (JSON over stdin/stdout)
    │   ├── carapace → completions (1000+ commands)
    │   ├── bash → POSIX scripts
    │   ├── Python/xonsh → data processing
    │   ├── babashka → Clojure scripting
    │   ├── Deno/Node → npm ecosystem
    │   └── any binary → custom tools
    │
    └── Replaceable layers
        ├── Editing mode (readline, vi, vim, emacs, helix)
        ├── Completions (carapace, bash fallback)
        ├── History backend (SQLite)
        └── Prompt (powerline, custom spans)
```

## Phase 1: Foundation

### 1.1 Enhanced History ✅ DONE
SQLite-backed history with git branch, exit status, duration tracking.
Branch: `feature/enhanced-history`

### 1.2 Better Prompt ✅ DONE
New spans: last_status, duration, git_branch, git_status, time, date, load, env.
Branch: `feature/better-prompt`

### 1.3 SQLite History ✅ DONE
Replaced TSV with SQLite for indexed queries.
Branch: `feature/sqlite-history`

### 1.4 Carapace Completions
Replace bash_server.rs with carapace as primary completer.
- JSON protocol for structured completions
- 1000+ commands out of the box
- bash_server as fallback for unknown commands

### 1.5 Deep Path Completion
Murex-style deep completion: type `nsh` → finds `~/nsh/src/main.rs`.
- Full directory tree search (not just next level)
- Fuzzy matching across path components
- SQLite index for speed (like file index)
- Respect .gitignore by default, Alt+A to show all

## Phase 2: Interactive Experience

### 2.1 Editing Modes
Swappable editing layers, each complete and self-contained:

| Mode | Description |
|------|-------------|
| **readline** | Default. Ctrl+A/E/K/Y/W/U, Meta+B/F |
| **vi** | Classical. hjkl, i/Esc, basic motions |
| **vim** | Full. text objects, macros, registers, visual |
| **emacs** | C-x prefix, minibuffer, region highlighting |
| **helix** | Selection-first, multiple cursors |

### 2.2 Hint Text
Murex-style contextual help below the prompt:
```
~/projects $ cat
/usr/bin/cat - concatenate files and print on the standard output
```
- Show command path + description from `whatis`/man pages
- Cached in SQLite for speed
- Update live as you type

### 2.3 Syntax Highlighting
Real-time highlighting as you type:
- Commands in green
- Flags in yellow
- Strings in cyan
- Errors in red (underlined)
- Paths in blue

### 2.4 Autosuggestions
Fish-style inline suggestions from history:
- Ghost text shows last matching command
- Ctrl+F or → to accept
- Weighted by recency and frequency

### 2.5 Safer Pasting
Murex-style paste safety:
- Multi-line paste shows warning prompt
- Option to preview contents before executing
- Protects against clipboard injection attacks

## Phase 3: Extension Bridge

### 3.1 JSON Protocol
Extensions communicate via JSON over stdin/stdout:
```json
// Request
{"command": "kubectl", "words": ["kubectl", "get", "pods"], "current_word": 3}

// Response
{"completions": [{"text": "pod-1", "kind": "pod"}]}
```

### 3.2 Extension Registry
`~/.config/nsh/extensions.json`:
```json
{
  "completions": {
    "carapace": {"command": "carapace", "priority": 1},
    "bash": {"command": "bash-completion", "priority": 2}
  },
  "scripts": {
    "python": {"command": "python3", "extensions": [".py"]},
    "clojure": {"command": "babashka", "extensions": [".clj"]}
  }
}
```

### 3.3 Typed Pipes (Optional)
Murex-inspired dual-channel pipelines:
- `|` — byte channel (text, traditional)
- `->` — value channel (structured data, when available)
- Commands can opt-in to structured output
- Graceful fallback to text when structured isn't available

## Phase 4: File Index

### 4.1 Background Indexer
Lightweight file index (like Baloo but simpler):
```sql
CREATE TABLE file_index (
    path TEXT PRIMARY KEY,
    name TEXT,
    mtime INTEGER,
    atime INTEGER,
    depth INTEGER
);
```
- Updated on cd, ls, file operations
- Respects .gitignore by default
- Used for deep path completion
- Frecency ranking (recent + frequent paths)

### 4.2 Smart Filtering
- Default: skip .git/, target/, node_modules/, __pycache__/
- Shift+Tab: show all results including ignored files
- `find:` prefix: explicit search across everything

## Phase 5: Polish

### 5.1 Better Error Messages
Murex-style contextual errors:
```
nsh: command not found: gti
    Did you mean: git?
    
nsh: permission denied: /etc/shadow
    Try: sudo cat /etc/shadow
```

### 5.2 Command Preview
F9 previews command output while typing:
- Safe commands auto-execute (cat, grep, jq)
- Unsafe commands require confirmation (rm, mv)
- Cache per-command, re-run from changed parameter

### 5.3 Cross-Platform Polish
- macOS: proper PATH handling, homebrew integration
- Windows: WSL support, PowerShell fallback for Windows commands
- FreeBSD/OpenBSD: job control, signal handling

## Configuration

Minimal config file: `~/.config/nsh/nshrc`

```sh
# Editing mode
set edit-mode readline  # or vi, vim, emacs, helix

# Prompt
set prompt powerline

# History
set history-size 10000

# Extensions
extension carapace
extension bash-fallback

# Hint text
set hint-text on
```

No plugins to install. No conflicts to resolve. Just settings.

## File Organization

```
src/
├── main.rs              # Entry point
├── mainloop.rs          # Event loop, TUI, input handling
├── shell.rs             # Shell state (env, aliases, history, jobs)
├── eval.rs              # Command evaluation
├── parser.rs            # Shell grammar (PEG)
├── history.rs           # History storage (SQLite)
├── history_search.rs    # History search and filtering
├── prompt.rs            # Prompt rendering and spans
├── completion.rs        # Completion engine (TODO)
├── carapace_server.rs   # Carapace integration (TODO)
├── bash_server.rs       # Bash completion fallback
├── expand.rs            # Variable/command expansion
├── highlight.rs         # Syntax highlighting
├── process.rs           # Process management
├── file_index.rs        # Background file indexer (TODO)
├── edit_mode.rs         # Editing mode trait (TODO)
├── edit_readline.rs     # Readline editing (TODO)
├── edit_vi.rs           # Vi editing (TODO)
├── edit_vim.rs          # Vim editing (TODO)
├── edit_emacs.rs        # Emacs editing (TODO)
├── edit_helix.rs        # Helix editing (TODO)
└── extension.rs         # Extension bridge protocol (TODO)
```

## Milestones

- [x] **v0.5.0** — Enhanced history (git branch, exit status, duration)
- [x] **v0.5.1** — Better prompt (new spans)
- [x] **v0.5.2** — SQLite history backend
- [ ] **v0.6.0** — Syntax highlighting + autosuggestions
- [ ] **v0.7.0** — Carapace completions + deep path completion
- [ ] **v0.8.0** — Hint text + safer pasting + better errors
- [ ] **v0.9.0** — Typed pipes + file index
- [ ] **v1.0.0** — The best default shell (login shell, stable API, cross-platform)

**Post-1.0:**
- [ ] **v1.1.0** — Extension bridge (JSON protocol)
- [ ] **v1.2.0** — Editing modes (vi, vim, emacs, helix)
- [ ] **v1.3.0** — Command preview (F9 live output)
- [ ] **v1.4.0** — AI integration
