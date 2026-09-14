# zcoder Architecture Overview

## Intent

- Single entry point document for the `zcoder` architecture. Read this first to get the overall picture.

- Summarizes the Cargo workspace, crate responsibilities, dependency direction, runtime flow, and shared event contracts.

- Then drill into the crate specific specs for detail:

  - `dev/specs/spec-zcoder.md`: root binary crate, CLI parsing, startup orchestration, logging, and error conversion.
  - `dev/specs/spec-zc-common.md`: shared pure types and small utilities.
  - `dev/specs/spec-zc-core.md`: execution engine, config, prompts, model layer, and the AI file-change workflow.
  - `dev/specs/spec-zc-tui.md`: terminal UI overview, module boundaries, lifecycle, and runtime flow.
  - `dev/specs/spec-zc-tui-core.md`: TUI core runtime, events, app state, and event handling.
  - `dev/specs/spec-zc-tui-view.md`: TUI layout, section views, styles, and render helpers.
  - `dev/specs/spec-zc-asset.md`: embedded asset runtime and `.zcoder` workspace materialization.

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

## Crate Summary

- `zcoder` (root bin `zc`): CLI parsing, startup orchestration, debug logging, and orchestration-level error conversion. See `spec-zcoder.md`.

- `zc-common`: shared pure types and small utilities that cross crate boundaries. See `spec-zc-common.md`.

- `zc-core`: execution behavior, AI provider calls, config, prompts, model persistence, and file-change workflow. See `spec-zc-core.md`.

- `zc-tui`: terminal UI lifecycle, app state, event handling, and rendering. See `spec-zc-tui.md`, `spec-zc-tui-core.md`, and `spec-zc-tui-view.md`.

- `zc-asset`: embedded asset runtime used to materialize the `.zcoder` workspace. See `spec-zc-asset.md`.

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
