# zc-core Specification

## Intent

Define the execution behavior of the `zc-core` crate: AI provider calls, workspace context, file-change extraction and application, Lua script execution, configuration, prompts, and model persistence.

`zc-core` is the execution engine. The TUI and the root binary never perform long-running work themselves; they send typed commands and react to typed events.

## Module Layout

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

`lib.rs` re-exports `Config`, `ConfigManager`, and `Db`, and exposes the `exec` and `model` modules.

## Config

- `Config` is a cheap, cloneable handle over `Arc<ConfigInner>` with builder style `with_*` and `append_*` helpers.
- `ConfigManager` loads `.zcoder/config.toml`, supports `refresh_if_modified()` hot reload, and exposes `get_config()`.
- The default config defines `[workspace] working_dir`, `[maestro] model`, `[model_sizes]`, and `[model_aliases]`.
- `get_model(ref_name)` resolves size presets such as `$small`, alias chains, and reasoning suffixes such as `-low`, `-high`, and `-max`, with cycle detection.

## Prompts

- `maestro_entry_system(script_engine)` generates the system prompt.
- The prompt combines the UDIFFX file-change instructions with the generated AIPROG Lua API documentation.
- The generated system prompt is cached for inspection.

## Exec

- `Executor::new(config)` creates the command and event channels, syncs workspace assets, loads config, builds the AIPROG registry and script engine, composes the system prompt, and builds the base `ChatRequest`.
- `Executor::start()` consumes `ExecCmd` values until the command channel closes.
- `ExecutorConfig` carries `wspace_dir`, an optional `base_dir`, and an optional explicit `model`.

Executor command and event channels are aliased as `ExecCmdTx`/`ExecCmdRx` and `ExecEventTx`/`ExecEventRx`.

```rust
pub enum ExecCmd {
	RunPrompt(String),
}

pub enum ExecEvent {
	RunStart(Id),
	RunEnd(Id),
	RunError(Id),
}
```

## Model

- `ModelManager` is a `OnceLock` singleton providing the shared SQLite `Db`.
- Entities expose `ForCreate` and `ForUpdate` types with `*Bmc` accessors, such as `RunBmc` and `AirBmc`.
- `ModelEvent` values are published through the model bus so the TUI can react to entity changes without polling.
- `trim()` deletes run rows and is designed to be called at the start of a run; `db_size()` reports database size.

```rust
pub struct ModelEvent {
	entity: EntityType,   // Run, Aixc, ...
	action: EntityAction, // Created, Updated, ...
	id: Option<Id>,
	rel_ids: RelIds,
}
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
- `ModelManager` is a process-wide singleton created with `OnceLock` and exposed through `get_model_manager()`.
- Entities include runs (`RunBmc`) and AI exchanges (`AirBmc`). The `Aixc` entity type maps to AI exchange rows in model events.
- Model events are published on entity changes and forwarded into the TUI event stream by the TUI model loop.

## Error Ownership

`zc-core::Error` covers executor, config, provider, filesystem, and file-change application failures. External errors that occur inside execution behavior are converted into `zc_core::Error` inside `zc-core`, not in the root binary.

## Design Considerations

- `zc-core` owns execution because AI calls, file context loading, and file-change application are one cohesive workflow. Keeping that workflow in one crate avoids leaking execution internals into the UI or binary startup layer.
- `zc-core` depends on `zc-common` for channel primitives, cache, time, and yaml helpers, and on `zc-asset` for workspace asset sync.
- The executor boundary keeps long-running work, AI calls, and file changes out of the UI loop.
