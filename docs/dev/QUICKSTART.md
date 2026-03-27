# Developer Quickstart

## Build
```sh
cargo build
```

## Run
```sh
./target/debug/nsh
```

## Test
```sh
cargo test
```

## Architecture Overview

```
main.rs
  └── interactive_mode() → Mainloop::run()
        ├── crossterm (key events)
        ├── context_parser (parse input for completion/highlight)
        ├── highlight (syntax highlighting)
        ├── bash_server (completion bridge)
        └── shell.run_str() → parser → eval
```

## Key Files

| File | Purpose |
|------|---------|
| `mainloop.rs` | REPL, key handling, completion UI |
| `shell.rs` | Shell state (vars, aliases, jobs) |
| `parser.rs` | PEG parser (POSIX grammar) |
| `eval.rs` | Execute AST |
| `history.rs` | Command history |
| `prompt.rs` | Prompt rendering |
| `bash_server.rs` | Bash completion bridge |

## Adding a New Feature

1. Find the relevant file(s)
2. Make changes
3. Add tests in `#[cfg(test)]` module or `tests/`
4. Run `cargo test`
5. Build and test manually

## Code Style

- Follow Rust idioms
- Use `trace!()` for debug logging
- Handle errors with `Result<T, Error>`
- Keep POSIX compatibility
