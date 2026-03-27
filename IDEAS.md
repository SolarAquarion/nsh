# nsh Ideas & Design Philosophy

_Collected from shell research and design discussions._

## Core Philosophy

### The Extension Bridge

nsh is a **platform**, not a language. The shell provides the interactive experience (REPL, TUI, completions, history). The scripting language lives outside.

```
nsh (Rust binary)
    ├── Built-in: basic commands, pipes, variables, functions
    ├── Extension bridge: JSON protocol over stdin/stdout
    │   ├── bash → POSIX scripts
    │   ├── Python/xonsh → data processing, APIs
    │   ├── Coconut → functional programming
    │   ├── Node/Deno → npm ecosystem
    │   └── carapace → completions (1000+ commands)
    └── Replaceable layers:
        ├── Editing mode (readline, vi, vim, emacs, helix)
        ├── Completions (carapace, bash, fish)
        ├── History backend (SQLite)
        └── Prompt (powerline, custom)
```

### Design Principles

1. **Great defaults** — install and immediately enjoy, no config needed
2. **Optional replaceability** — every layer is swappable
3. **Extending power** — call any tool, any language
4. **Language-agnostic** — extensions don't depend on nsh's language
5. **Cross-platform** — runs on any OS that runs Rust

## What Makes nsh Different

### Not another shell language

Elvish, Ion, Nushell, Murex, Oh — all try to be the entire shell language. nsh is the platform that lets you choose the language.

### The interactive experience is the product

- Fast startup (<10ms, Rust single binary)
- Syntax highlighting (built-in, not a plugin)
- Autosuggestions from history (fish-style)
- Smart completions (carapace + fuzzy matching)
- SQLite history (indexed, searchable)
- Powerline prompt (git-aware, extensible)

### The TUI layer owns the editing experience

Unlike readline/prompt_toolkit, nsh controls the entire rendering pipeline:

- **Cursor shape changes** — block in normal, line in insert, underline in replace
- **Mode indicators** — `-- NORMAL --` in status line
- **Visual selection highlighting** — selected text highlighted
- **Command line** — `:` prompt for ex commands (vim mode)
- **Search highlighting** — matches highlighted as you type

No Athame-style hacks. No embedding vim. Just proper TUI control.

## Editing Modes

Each mode is a complete, self-contained editing experience:

| Mode | Description | Trigger |
|------|-------------|---------|
| **readline** | Default, universal, Ctrl+A/E/K/Y/W/U, Meta+B/F | Default |
| **vi** | Classical, simple motions, hjkl, i/Esc | `set edit-mode vi` |
| **vim** | Full power, text objects, macros, registers, visual | `set edit-mode vim` |
| **emacs** | C-x prefix, minibuffer, region highlighting | `set edit-mode emacs` |
| **helix** | Selection-first, multiple cursors, `x` to select line | `set edit-mode helix` |

### Vi-mode (classical)
- Basic motions: h, j, k, l, w, b, e, 0, $
- Insert/Normal mode toggle
- Basic operators: d, c, y

### Vim-mode (full)
- Motions: w, b, e, 0, $, f{char}, t{char}, %, gg, G
- Text objects: ciw, da", vi(, ci{, dit
- Operators: d, c, y, >, <, gU, gu
- Visual mode: v, V, Ctrl+V
- Registers: "ay, "ap, "+y (clipboard)
- Macros: qa...q, @a, @@
- Search: /, ?, n, N, *, #
- Marks: ma, 'a, `a`
- Repeat: ., ;, ,
- Undo/redo: u, Ctrl+R with undo tree

## Extension Protocol

### JSON over stdin/stdout

```json
// Request (nsh → extension)
{
  "command": "kubectl",
  "words": ["kubectl", "get", "pods", "-n", ""],
  "current_word": 4,
  "current_word_partial": ""
}

// Response (extension → nsh)
{
  "completions": [
    {"text": "default", "display": "default (current)", "kind": "namespace"},
    {"text": "kube-system", "display": "kube-system", "kind": "namespace"}
  ]
}
```

### Why JSON?
- Language-agnostic (every language can parse JSON)
- Human-readable (debug with `cat`)
- Structured (no text parsing)
- Standard (no custom format to learn)

## Ideas Stolen From Other Shells

### From Elvish
- **Dual-channel pipelines** — byte channel (text) + value channel (structured data)
- **`put` vs `echo`** — `put` preserves structure, `echo` converts to string
- **First-class lists** — lists are values, not string-split text

### From PowerShell
- **Object pipelines** — structured data flowing through pipes
- **Property access** — `command.output.property` without parsing
- **Structured returns** — not just exit codes, but full result objects

### From Ion
- **Sigil-based typing** — `$var` for strings, `@array` for arrays
- **Built-in string methods** — `$var:to_upper`, `$var:split ","`
- **Not POSIX** — freedom to design a better language
- **Performance focus** — minimal heap allocations, minimal CPU cycles

### From Ksh93
- **Compound variables** — shell objects (structs)
- **Active variables** — variables that trigger functions on read/write
- **Name references** — pointers for variables
- **C API for extensions** — extensibility via native code

### From Murex
- **Typed pipes** — `->` for structured data, `|` for legacy text
- **Hint text** — status line below prompt with contextual info
- **Preview completions** — F1 shows man pages, file contents, images
- **Preview command lines** — F9 runs pipeline and shows output while typing
- **Safer pasting** — warning on multi-line paste
- **Deep path completion** — find `nsh` and it matches `~/nsh/src/main.rs`, not just next level
- **Smart autocompletion by pipe type** — `->` suggests methods that accept the previous command's output type
- **Inline spellchecker** — underlines errors as you type
- **JSON-schema completions** — define completion rules as JSON, not scripts
- **AllowSubstring matching** — `da` matches `Monday`, `Tuesday` (substring, not prefix)
- **AutoBranch** — completion automatically traverses directory branches
- **Dynamic completions with caching** — completions can be computed dynamically with configurable TTL
- **Smarter error messages** — detailed, contextual errors instead of cryptic failures
- **Custom readline library** — Murex wrote their own readline instead of using the system one

### From Oh
- **First-class channels** — pipes as values you can store and pass
- **Rich return values** — structured returns, not just exit codes
- **No word splitting** — lists are first-class, strings don't split
- **Lexical scope** — proper scoping rules
- **Kernel-style fexprs** — define new language constructs

### From Fish
- **Autosuggestions** — inline suggestions from history as you type
- **Syntax highlighting** — colors as you type, not after execution
- **Scripting breakage lesson** — don't write extensions in the shell language

### From Oil Shell
- **Better bash** — cleaned up syntax, but still bash at core
- **Lesson** — being a better bash is limiting; be a better shell

### From Athame
- **Full vim in shell** — embeds real vim process for keybindings
- **Lesson** — vi-mode emulations are always incomplete; do it properly in Rust

### From Xonsh
- **Python in shell** — full Python language available
- **Lesson** — powerful language but limited by Python's runtime constraints
- **Coconut integration** — functional programming compiles to Python

## Prompt Features

### New spans (already implemented)
| Span | Description |
|------|-------------|
| `{last_status}` | Exit status (red if non-zero) |
| `{last_duration}` | Duration (human readable) |
| `{time}` | Current time HH:MM:SS |
| `{date}` | Current date YYYY-MM-DD |
| `{git_branch}` | Git branch name |
| `{git_status}` | * if uncommitted changes |
| `{load}` | System load average |
| `{env:VAR}` | Environment variable |

### Planned spans
| Span | Description |
|------|-------------|
| `{kubernetes}` | Current k8s context/namespace |
| `{docker}` | Docker context |
| `{aws}` | AWS profile |
| `{ssh}` | SSH connection info |
| `{battery}` | Battery level |
| `{memory}` | Memory usage |
| `{cpu}` | CPU usage |

## History System

### Current (already implemented)
- SQLite backend with indexed queries
- Git branch, exit status, duration tracking
- Fuzzy search via FuzzyVec
- Automatic migration from TSV format

### Planned
- **Full-text search** — SQLite FTS5 for command search
- **Session tracking** — group commands by session
- **Statistics** — most used commands, average duration
- **Export/import** — JSON export for backup
- **Encryption** — optional encrypted history for sensitive commands

## Completion System

### Current
- bash_server.rs (shells out to bash for completions)
- carapace integration (1000+ commands, JSON protocol)

### Planned
- **carapace as primary** — fast, structured, JSON output
- **bash_server as fallback** — for commands carapace doesn't know
- **LSP-style completions** — rich completion with documentation
- **Context-aware** — completions based on current command, directory, git state
- **Custom completions** — user-defined completion specs via JSON

## Post-POSIX Direction

nsh is Post-POSIX. When you need POSIX scripts, run bash.

### What this enables
- Modern syntax (`if { }` blocks instead of `if/then/fi`)
- Structured data (objects in pipelines)
- Proper error handling (Result types, not exit codes)
- First-class functions (closures, lambdas)
- Real data structures (lists, maps, not just strings)

### What this means practically
```
nsh (interactive daily driver)
    ├── native: history, completions, prompt, syntax highlighting
    ├── bash: POSIX scripts (when needed)
    ├── xonsh: Python scripts (complex automation)
    └── coconut: functional programming (when you need it)
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                           main.rs                               │
│  Parse CLI args, init logger, load config, call Mainloop::run() │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                         mainloop.rs                             │
│  • Event loop (keyboard, resize, signals, completions)          │
│  • Command input handling (readline/vi/vim/emacs/helix)         │
│  • Completion UI (inline, menu, preview)                        │
│  • History navigation (up/down, Ctrl+R search)                  │
│  • Syntax highlighting (real-time, as-you-type)                 │
│  • Command execution (delegates to Shell::run_str)              │
│  • Prompt rendering (powerline, custom spans)                   │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                          shell.rs                               │
│  • Environment variables, aliases, functions                    │
│  • History (SQLite-backed)                                      │
│  • Job control (foreground/background)                          │
│  • Path table (command lookup)                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                           eval.rs                               │
│  • Command evaluation (builtins, external, functions)           │
│  • Pipeline execution (text channel + value channel)            │
│  • Redirections, backgrounding                                  │
└──────────────────────────────┬──────────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────────┐
│                      extension bridge                            │
│  • carapace completions (JSON protocol)                         │
│  • bash_server fallback (for unknown commands)                  │
│  • Custom extensions (user-defined JSON protocol)               │
└─────────────────────────────────────────────────────────────────┘
```

## File Organization

```
src/
├── main.rs              # Entry point, CLI args, config loading
├── mainloop.rs          # Event loop, input handling, TUI
├── shell.rs             # Shell state (env, aliases, history, jobs)
├── eval.rs              # Command evaluation and execution
├── parser.rs            # Shell grammar parser (PEG)
├── history.rs           # History storage (SQLite)
├── history_search.rs    # History search and filtering
├── prompt.rs            # Prompt rendering and spans
├── completion.rs        # Completion engine (TODO)
├── carapace_server.rs   # Carapace completion integration (TODO)
├── bash_server.rs       # Bash completion fallback
├── expand.rs            # Variable/command expansion
├── highlight.rs         # Syntax highlighting
├── process.rs           # Process management and job control
├── builtins/            # Built-in commands
├── fuzzy.rs             # Fuzzy matching
├── theme.rs             # Color themes
├── dircolor.rs          # Directory colors
└── context_parser.rs    # Input context for completion/highlighting
```

## References

### Shells studied
- [Elvish](https://elv.sh/) — dual-channel pipelines, structured data
- [Ion](https://doc.redox-os.org/ion-manual/) — Rust shell, sigil typing, Post-POSIX
- [Murex](https://murex.rocks/) — typed pipes, hint text, preview completions
- [Nushell](https://www.nushell.sh/) — typed data pipelines
- [Oil Shell](https://www.oilshell.org/) — better bash
- [Oh](https://github.com/michaelmacinnis/oh) — first-class channels, fexprs
- [PowerShell](https://learn.microsoft.com/en-us/powershell/) — object pipelines
- [Ksh93](https://github.com/ksh93/ksh) — compound variables, active variables
- [Fish](https://fishshell.com/) — autosuggestions, syntax highlighting
- [Xonsh](https://xon.sh/) — Python in shell
- [Coconut](https://coconut-lang.org/) — functional Python
- [Athame](https://github.com/ardagnir/athame) — full vim in shell

### Tools
- [carapace](https://github.com/rsteube/carapace-bin) — multi-shell completion engine
- [prompt_toolkit](https://python-prompt-toolkit.readthedocs.io/) — Python TUI library
- [crossterm](https://github.com/crossterm-rs/crossterm) — Rust terminal library
