# zcoder Root Binary Specification

## Intent

Define the responsibilities, startup flow, CLI surface, logging, and error model of the `zcoder` root binary crate (bin `zc`).

The root binary is a thin startup shell. It parses command-line input, builds the executor configuration, spawns the executor, and hands control to the TUI. It does not own executor workflow logic, AI provider calls, file-change application, or TUI state and rendering.

## Module Layout

```text
src/
  main.rs   # startup orchestration and tracing setup
  cmd.rs    # CLI parsing with clap
  error.rs  # root crate Error and Result
```

## Responsibilities

- parse command-line input through `CliCmd`
- derive startup configuration values such as the base directory
- construct `zc_core::exec::ExecutorConfig`
- create the executor, executor sender, and status receiver through `zc_core::exec::Executor`
- spawn the executor task
- start the terminal UI through `zc_tui::start_tui`
- own only orchestration-level error conversion
- own debug logging setup

The root binary should not own:

- executor workflow logic
- AI provider calls
- file loading or file-change application
- terminal UI state, rendering, or event handling
- shared event data definitions

## CLI

- `CliCmd` in `src/cmd.rs` is parsed with `clap`.
- It exposes an optional `prompt` positional and an optional `--dir` flag.

## Startup Sequence

```text
root main
  -> parse CLI
  -> resolve wspace_dir as the current directory
  -> build ExecutorConfig::default().with_wspace_dir(wspace_dir)
  -> apply .with_base_dir(dir) when --dir is given
  -> Executor::new(config) -> (Executor, ExecCmdTx, ExecEventRx)
  -> tokio::spawn(executor.start())
  -> zc_tui::start_tui(executor_tx, status_rx, cli_cmd.prompt).await
```

## Logging

- Debug tracing is written to `.zcoder/debug-log/log.txt` through a non-blocking file appender.
- The tracing subscriber is configured with an `EnvFilter` that enables `debug` for the application crates.
- The non-blocking guard is kept alive for the process lifetime so buffered logs are flushed.

## Error Model

`src/error.rs` owns the root crate `Error` and `Result`. It covers orchestration-level conversion only:

- `Custom`
- `SimpleFs`
- `ZcCore(zc_core::exec::Error)`
- `ZcTui(zc_tui::Error)`

Errors are converted at the crate boundary where they originate. The root binary converts errors from the crate entry points it calls directly and does not convert errors from implementation dependencies it no longer calls directly.

## Design Considerations

- A thin root binary makes startup easy to audit and prevents the binary crate from accumulating domain behavior as the application grows.
- The root binary depends only on the crates it wires together, `zc-core` and `zc-tui`, so the dependency graph stays explicit.
- Terminal lifecycle and long-running work stay outside the binary, which keeps `main.rs` small and stable.
