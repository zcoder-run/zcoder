# zcoder Architecture Overview

## Intent

- Single entry point document for the `zcoder` architecture.

- Summarizes the Cargo workspace, crate responsibilities, dependency direction, runtime flow, event contracts, and the prompt execution pipeline.

- Complements the focused specs: `dev/specs/spec-workspace.md`, `dev/specs/spec-tui-general.md`, `dev/specs/spec-tui-core.md`, and `dev/specs/spec-tui-view.md`.

## Workspace Layout

```text
Cargo.toml              # root package, bin `zc`, workspace members and shared deps
src/
  main.rs               # startup orchestration
  cmd.rs                # CLI parsing with clap
  error.rs              # root crate error type
crates/
  zc-common/            # shared pure types and small utilities
  zc-core/              # execution engine and AI file-change workflow
  zc-tui/               # terminal UI runtime, state, and rendering
  zc-asset/             # embedded asset runtime
dev/specs/              # architecture and behavior specs
.zcoder/                # generated runtime dir (config, debug logs, cache)
```

## Dependency Direction

```text
zcoder (root bin)
  -> zc-core
  -> zc-tui

zc-core
  -> zc-common
  -> zc-asset

zc-tui
  -> zc-common
  -> zc-core

zc-common -> no domain crates
zc-asset  -> no domain crates
```

- The root binary depends only on what it wires together: `zc-core` and `zc-tui`.

- `zc-tui` depends on `zc-core` for the executor command sender and event receiver, and on `zc-common` for channel primitives.

- `zc-core` depends on `zc-common` for channel primitives, cache, time, and yaml helpers, and on `zc-asset` for workspace asset sync.

- `zc-common` and `zc-asset` stay dependency light and never depend on domain crates.

## Crates

### Root Binary (`zcoder`, bin `zc`)

- Owns CLI parsing through `CliCmd` in `src/cmd.rs`: an optional `prompt` positional and an optional `--dir` flag.

- Owns process startup and hand-off to the TUI. It does not own executor workflow logic, AI provider calls, file change application, or TUI state and rendering.

- Owns debug logging setup: tracing output is written to `.zcoder/debug-log/log.txt` through a non blocking file appender.

- Owns only orchestration level error conversion in `src/error.rs`: `Custom`, `SimpleFs`, `ZcCore(zc_core::exec::Error)`, and `ZcTui(zc_tui::Error)`.

- `src/main.rs` startup sequence:

  - parse `CliCmd`
  - resolve `wspace_dir` as the current directory
  - build `ExecutorConfig::default().with_wspace_dir(wspace_dir)`
  - apply `.with_base_dir(dir)` when `--dir` is given
  - call `Executor::new(config)`, which returns `(Executor, ExecCmdTx, ExecEventRx)`
  - spawn the executor task with `tokio::spawn(executor.start())`
  - run `zc_tui::start_tui(executor_tx, status_rx, cli_cmd.prompt).await`

### zc-common

- Owns shared pure types and small utilities that cross crate boundaries.

- Modules:

  - `error`: local `Error` and `Result`, scoped to this crate only
  - `event_base`: bounded mpsc channel primitives, `MpscTx`, `MpscRx`, and `new_mpsc_bounded(name, capacity)`
  - `time`: time helpers such as `now_micro()`
  - `cache`: file cache helpers such as `save_file_cache(name, content)`
  - `yaml`: content conversion helpers such as `json_to_yaml_string`

- Must not own the workspace wide error type, executor behavior, TUI behavior, or application services.

### zc-asset

- Owns the embedded asset runtime used to materialize the `.zcoder` workspace directory.

- Embeds the asset archive at compile time through `ASSETS_ZIP`, sourced from the `ASSETS_ZIP` environment variable with `include_bytes!`.

- Public API:

  - `extract_asset(path)` and `extract_asset_str(path)`
  - `extract_zfile(path)` returning `ZFile { path, content }`
  - `list_asset_paths(prefix)`
  - `update_zcoder_project(project_dir)`

- `update_zcoder_project` creates `.zcoder/` in the target workspace and writes only missing assets, so user edits are preserved.

- Has no dependency on other domain crates.

### zc-core

- Owns execution behavior: AI provider calls, workspace context, file change extraction and application, and Lua script execution.

- Module tree:

```text
crates/zc-core/src/
  lib.rs               # module registry, re-exports Config/ConfigManager/Db
  derive_aliases.rs    # internal derive alias helpers
  config/
    config_impl.rs     # Config and ConfigInner
    manager.rs         # ConfigManager, file loading and hot reload
    error.rs
  exec/
    exec_event.rs      # ExecCmd, ExecEvent, and channel aliases
    executor.rs        # Executor, ExecutorConfig, run pipeline
    air_exec.rs        # provider call helper, exec_air_chat
    error.rs
  model/
    model_manager.rs   # process wide ModelManager singleton
    db.rs              # sqlite access
    bus.rs             # ModelEvent publication
    entities/          # air, common, and run entities with Bmc accessors
    types.rs           # Id, EpochUs, and shared model types
    support/
  prompts/
    prompts_maestro.rs # system prompt composition
```

- `config`:

  - `Config` is a cheap, cloneable handle over `Arc<ConfigInner>` with builder style `with_*` and `append_*` helpers.
  - `ConfigManager` loads `.zcoder/config.toml`, supports `refresh_if_modified()` hot reload, and exposes `get_config()`.
  - The default config defines `[workspace] working_dir`, `[maestro] model`, `[model_sizes]`, and `[model_aliases]`.
  - `get_model(ref_name)` resolves size presets such as `$small`, alias chains, and reasoning suffixes such as `-low`, `-high`, and `-max`, with cycle detection.

- `prompts`:

  - `maestro_entry_system(script_engine)` generates the system prompt.
  - The prompt combines the UDIFFX file change instructions with the generated AIPROG Lua API documentation.
  - The generated system prompt is cached for inspection.

- `exec`:

  - `Executor::new(config)` creates the command and event channels, syncs workspace assets, loads config, builds the AIPROG registry and script engine, composes the system prompt, and builds the base `ChatRequest`.
  - `Executor::start()` consumes `ExecCmd` values until the command channel closes.
  - `ExecutorConfig` carries `wspace_dir`, an optional `base_dir`, and an optional explicit `model`.

- `model`:

  - `ModelManager` is a `OnceLock` singleton providing the shared SQLite `Db`.
  - Entities expose `ForCreate` and `ForUpdate` types with `*Bmc` accessors, such as `RunBmc` and `AirBmc`.
  - `ModelEvent` values are published through the model bus so the TUI can react to entity changes without polling.
  - `trim()` deletes run rows and is designed to be called at the start of a run; `db_size()` reports database size.

### zc-tui

- Owns the terminal UI lifecycle, app state, event handling, and rendering.

- Module tree:

```text
crates/zc-tui/src/
  lib.rs
  error.rs
  core/
    tui_impl.rs           # terminal setup, channel wiring, task startup
    tui_loop.rs           # draw then handle events
    tui_event_handlers.rs # terminal, action, exec, and model event handling
    event.rs              # TuiEvent and AppActionEvent
    debouncer.rs          # coalescing of bursty events
    term_reader.rs        # terminal input task
    ping_timer.rs         # periodic tick task
    model_loop.rs         # model bus to TuiEvent::Model forwarder
    sys_state.rs          # process and database metrics snapshots
    tui_state/            # TuiState and StateProcessor
    types.rs              # shared TUI enums such as scroll identifiers
  view/
    main_view.rs          # full screen layout
    answer_view.rs        # answer or error content area
    status_view.rs        # status line
    prompt_view.rs        # prompt input area
    footer_view.rs        # key hints
    style.rs              # shared style constants and helpers
    tblock.rs             # shared block helpers
```

- `start_tui(executor_tx, exec_rx, initial_prompt)` is the only public entry point.

- Terminal lifecycle: `ratatui::init()`, mouse capture, `terminal.clear()`, then `ratatui::restore()` and mouse capture release after the loop exits.

- The UI loop draws before handling each event so every applied event becomes visible on the next iteration.

- The TUI does not perform long running work. It sends typed `ExecCmd` values and reacts to `ExecEvent` and `ModelEvent` values.

## Event Contracts

```rust
// zc-core::exec
pub enum ExecCmd {
	RunPrompt(String),
}

pub enum ExecEvent {
	RunStart(Id),
	RunEnd(Id),
	RunError(Id),
}

// zc-core::model
pub struct ModelEvent {
	entity: EntityType,   // Run, Aixc, ...
	action: EntityAction, // Created, Updated, ...
	id: Option<Id>,
	rel_ids: RelIds,
}

// zc-tui::core
pub enum TuiEvent {
	Term(Event),
	Action(AppActionEvent),
	Exec(ExecEvent),
	Model(ModelEvent),
	Tick(i64),
	DoRedraw,
}

pub enum AppActionEvent {
	Quit,
	RunPrompt(String),
}
```

- Executor command and event channels are aliased as `ExecCmdTx`/`ExecCmdRx` and `ExecEventTx`/`ExecEventRx`.

- All UI signals share one `TuiEvent` stream so terminal input, actions, executor status, model updates, and ticks stay ordered in the UI loop.

## Runtime Flow

```text
CLI parse (zcoder)
  -> ExecutorConfig (wspace_dir, optional base_dir, optional model)
  -> Executor::new -> (Executor, ExecCmdTx, ExecEventRx)
  -> tokio::spawn(Executor::start())
  -> zc_tui::start_tui(ExecCmdTx, ExecEventRx, initial_prompt)
       -> tui_impl: ratatui init, TuiEvent channel, model loop, exec forwarder,
                    terminal reader, ping timer
       -> tui_loop: draw -> recv TuiEvent -> debounce -> handle
```

Action flow:

```text
Terminal input -> TuiEvent::Term -> tui_event_handlers
App intent     -> TuiEvent::Action -> state + ExecCmdTx -> Executor
Executor       -> TuiEvent::Exec -> state update
Model change   -> TuiEvent::Model -> state update
Timer          -> TuiEvent::Tick -> state update
```

## Execution Pipeline

The `RunPrompt` path spans the TUI, the executor, and the model layer.

1. The user presses `Enter` in the prompt view, and the TUI emits `AppActionEvent::RunPrompt(prompt)`.
2. `handle_app_action` marks the state as running and sends `ExecCmd::RunPrompt(prompt)` to the executor.
3. `handle_run_prompt` creates a `Run` row and emits `ExecEvent::RunStart(run_id)`.
4. Workspace assets are re-synced and the config is hot reloaded before each run.
5. The model is resolved from the explicit model, or from `[maestro] model` through `get_model`, and the base directory is resolved from `--dir`, `[workspace] working_dir`, or `wspace_dir`.
6. The user prompt is appended to the base chat request, and `exec_air_chat` performs the provider call while recording an `Air` row with timing, tokens, and cost.
7. Raw request and response payloads are cached to `.zcoder` cache files for inspection.
8. The response text is parsed for UDIFFX file changes, which are extracted and applied to the base directory.
9. The remaining text is parsed for `<AIPROG>` Lua scripts, which run through the AIPROG script engine with a directory context scoped to the base directory.
10. Script results and remaining text are combined into the final answer.
11. The `Run` row is updated with the answer and end state, and `ExecEvent::RunEnd(run_id)` is emitted.
12. On failure, the `Run` row is updated with the error and `ExecEvent::RunError(run_id)` is emitted.
13. `ModelEvent` values emitted during the run drive the TUI work info display, such as model name, elapsed time, tokens, and cost.

## Data and State

- Persistence uses SQLite through `rusqlite`.

- `ModelManager` is a process wide singleton created with `OnceLock` and exposed through `get_model_manager()`.

- Entities include runs (`RunBmc`) and AI exchanges (`AirBmc`). The `Aixc` entity type maps to AI exchange rows in model events.

- Model events are published on entity changes and forwarded into the TUI event stream by `model_loop`.

- TUI state is a pure render model: input buffer, waiting flag, status text, last prompt, last answer, last error, scroll positions, and optional system metrics.

## Error Ownership

- Each crate owns its own `Error` and `Result`.

- `zc-common::Error` covers shared utility failures and is not a workspace wide error.

- `zc-core::Error` covers executor, config, provider, filesystem, and file change application failures.

- `zc-tui::Error` covers terminal, UI, and lifecycle failures.

- The root binary converts errors from the crate entry points it calls directly.

## Design Considerations

- The workspace is split by runtime responsibility rather than by technology, so the executor and the TUI can evolve independently.

- A thin root binary makes startup easy to audit and prevents domain behavior from accumulating in the binary crate.

- A narrow `zc-common` avoids creating a large shared dependency that every crate must accept.

- One app event stream in the TUI keeps terminal input, actions, executor status, model events, and ticks consistently ordered.

- The executor boundary keeps long running work, AI calls, and file changes out of the UI loop.

- The asset runtime keeps workspace `.zcoder` state reproducible while preserving user edits.

- The module structure is intentionally modular so navigation, scrolling, popups, and richer run views can be added without turning `tui_loop.rs` or `main_view.rs` into catch all files.
