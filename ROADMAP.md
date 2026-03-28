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

## v1.0 Features

### Completion Engine
- Carapace as primary completer (1000+ commands, JSON protocol)
- SQLite file index (background, respects .gitignore)
- Deep path completion (type `nsh` → finds `~/nsh/src/main.rs`)
- Hint text (command path + description as you type)

### Interactive Polish
- Syntax highlighting (real-time, as you type)
- Autosuggestions (fish-style ghost text from history)
- Safer pasting (multi-line paste warning with preview)
- Better errors ("Did you mean: git?")

### Core Differentiator
- Typed pipes (`|` for text, `->` for structured data)
- Graceful fallback when structured isn't available

### Production
- Prompt API (powerlevel10k features: instant prompt, async segments, custom segments)
- Login shell (single Rust binary, zero dependencies)
- Cross-platform (Linux, macOS, FreeBSD, Windows/WSL)
- Stable API (config format, extension protocol)

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

**v1.0 — The Best Default Shell**

Work streams (parallel, ship when ready):

**A. Completion Engine**
- [x] SQLite history backend
- [ ] File index (SQLite)
- [ ] Carapace completions
- [ ] Deep path completion
- [ ] Hint text (whatis cache)

**B. Interactive Polish**
- [ ] Syntax highlighting
- [ ] Autosuggestions
- [ ] Safer pasting
- [ ] Better errors

**C. Core Differentiator**
- [ ] Typed pipes

**D. Production**
- [ ] Prompt API (powerlevel10k-native)
- [ ] Login shell
- [ ] Cross-platform
- [ ] Stable API

**Post-1.0:**
- Extension bridge (JSON protocol)
- Editing modes (vi, vim, emacs, helix)
- Command preview (F9)
- AI integration

Ship features when ready. Version as you go.
