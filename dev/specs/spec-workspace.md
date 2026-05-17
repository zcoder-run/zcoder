# Workspace Specification

## Intent

Define the crate boundaries, dependency direction, startup flow, and ownership model for the `zcoder` workspace.

The workspace separates the application into focused crates:

- the root binary crate owns CLI parsing and startup orchestration
- `zc-common` owns shared pure data types used across crates
- `zc-core` owns execution behavior and AI file-change workflow
- `zc-tui` owns terminal UI behavior and rendering

The intended visible runtime path is:

```rust
CliCmd::parse()
ExecutorConfig::default().with_base_dir(base_dir)
Executor::new(executor_config)
tokio::spawn(executor.start())
zc_tui::start_tui(executor_tx, status_rx, cli_cmd.prompt).await
```

The workspace design intentionally keeps execution logic, terminal UI logic, and shared data boundaries separate so each crate can evolve without turning the binary crate into an application-wide implementation container.

## Code Design

The workspace is organized as:

```text
.
  Cargo.toml
  src/
    main.rs
    cmd.rs
    error.rs
  crates/
    zc-common/
      Cargo.toml
      src/
        lib.rs
        error.rs
        event.rs
    zc-core/
      Cargo.toml
      src/
        lib.rs
        error.rs
        executor.rs
    zc-tui/
      Cargo.toml
      src/
        lib.rs
        error.rs
        core/
        view/
```

### Root binary crate

The root binary crate is a thin startup shell.

Responsibilities:

- parse command-line input through `CliCmd`
- derive startup configuration values such as the base directory
- construct `zc_core::ExecutorConfig`
- create the executor, executor sender, and status receiver through `zc_core::Executor`
- spawn the executor task
- start the terminal UI through `zc_tui::start_tui`
- own only orchestration-level error conversion

The root binary should not own:

- executor workflow logic
- AI provider calls
- file loading or file-change application
- terminal UI state, rendering, or event handling
- shared event data definitions

The root crate depends only on crates it uses directly for startup orchestration.

### zc-common

`zc-common` owns shared pure types that need to cross crate boundaries.

Current shared event types:

```rust
pub enum ExecActionEvent {
	Prompt(String),
	Shutdown,
}

pub enum ExecStatusEvent {
	Started,
	Progress(String),
	Done,
	Error(String),
}
```

Responsibilities:

- define shared event data contracts
- expose shared types from `lib.rs`
- keep its local `Error` and `Result` scoped to `zc-common` only
- stay dependency-light

`zc-common` should not become a global application foundation crate. It should not own the workspace-wide error type, executor behavior, TUI behavior, or application services.

### zc-core

`zc-core` owns execution behavior.

Responsibilities:

- define and expose `Executor`
- define and expose `ExecutorTx`
- define and expose `ExecutorConfig`
- create executor action and status channels
- receive `zc_common::ExecActionEvent`
- emit `zc_common::ExecStatusEvent`
- manage executor lifecycle
- handle prompt execution
- build base AI chat requests
- call AI provider APIs
- load file context
- extract file changes
- apply file changes
- convert execution dependency errors into `zc_core::Error`

`zc-core` uses its own local error model:

```rust
pub type Result<T> = core::result::Result<T, Error>;
```

External errors that occur inside execution behavior are converted into `zc_core::Error` inside `zc-core`, not in the root binary.

### zc-tui

`zc-tui` owns terminal UI behavior.

Responsibilities:

- expose `start_tui`
- initialize and restore the terminal
- own TUI app state
- own TUI event handling
- read terminal input
- forward user actions to `zc_core::ExecutorTx`
- receive executor status through `zc_common::ExecStatusEvent`
- render terminal views
- convert terminal and UI errors into `zc_tui::Error`

The public entry point is:

```rust
pub async fn start_tui(
	executor_tx: ExecutorTx,
	status_rx: ExecutorStatusRx,
	initial_prompt: Option<String>,
) -> Result<()>;
```

`zc-tui` should not perform long-running execution work, AI provider calls, file loading, extraction, application, installs, or checks. It coordinates UI state and sends typed actions to `zc-core`.

### Dependency direction

The intended dependency direction is:

```text
root binary
  -> zc-core
  -> zc-tui

zc-core
  -> zc-common

zc-tui
  -> zc-common
  -> zc-core

zc-common
  -> no domain crates
```

The root binary depends on both `zc-core` and `zc-tui` because it wires the runtime together.

`zc-core` depends on `zc-common` for shared event contracts.

`zc-tui` depends on `zc-common` for status event contracts and on `zc-core` for `ExecutorTx` and `ExecutorStatusRx`.

`zc-common` must not depend on `zc-core`, `zc-tui`, or the root binary.

### Runtime flow

Startup flow:

```text
root main
  -> parse CLI
  -> build ExecutorConfig
  -> create Executor, ExecutorTx, status_rx
  -> spawn Executor::start()
  -> call zc_tui::start_tui(executor_tx, status_rx, initial_prompt)
```

Action flow:

```text
User input
  -> zc-tui AppEvent
  -> zc-core ExecutorTx
  -> zc-common ExecActionEvent
  -> zc-core Executor
```

Status flow:

```text
zc-core Executor
  -> zc-common ExecStatusEvent
  -> zc-tui app event stream
  -> zc-tui state update
  -> zc-tui render
```

### Error ownership

Each crate owns its own error type.

- root binary error covers startup orchestration and conversions from crate entry points it directly calls
- `zc-common::Error` is local to shared common behavior and is not a workspace-wide error
- `zc-core::Error` covers executor, AI, filesystem, and file-change application failures
- `zc-tui::Error` covers terminal, UI, and TUI lifecycle failures

Errors should be converted at the crate boundary where they originate. The root binary should not convert errors from implementation dependencies it no longer calls directly.

## Design Considerations

The workspace is split by runtime responsibility rather than by technology alone. This keeps the root binary small while allowing execution and terminal UI behavior to evolve independently.

A thin root binary makes startup easy to audit. It also prevents the binary crate from accumulating domain behavior as the application grows.

`zc-common` is intentionally narrow. Keeping it limited to pure shared contracts avoids creating a large shared dependency that every crate must accept. This reduces coupling and keeps ownership decisions explicit.

`zc-core` owns execution because AI calls, file context loading, and file-change application are one cohesive workflow. Keeping that workflow in one crate avoids leaking execution internals into the UI or binary startup layer.

`zc-tui` owns terminal behavior because terminal lifecycle, input reading, app state, and rendering form a separate runtime concern. The UI communicates with execution through typed events and sender boundaries rather than direct workflow calls.

The per-crate error model preserves clear ownership. Each crate converts errors from the dependencies it directly uses, and public entry points expose only that crate's error type.

The dependency direction allows the UI and executor to share event contracts without creating cyclic ownership. `zc-common` provides the stable shared data boundary, while `zc-core` and `zc-tui` remain responsible for their own behavior.
