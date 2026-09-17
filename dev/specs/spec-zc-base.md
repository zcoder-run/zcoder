# zc-base Specification

## Intent

Define the base role: the implementation owner for persistence and execution.

`zc-base` owns the SQLite database, the generic CRUD support, `ModelManager`, the model bus, the `RunBmc`/`AirBmc` accessors, the configuration, the prompts, and the executor. It also owns the Core-facing pump loops and the model RPC handler that serve the `zc-router` contract.

Only `zc-base` opens a `rusqlite::Connection` or runs SQL.

## Module Layout

```text
crates/zc-base/src/
  lib.rs           # ZcBaseConfig, start_base_core, InProcBase, ZcBase
  config/          # Config and ConfigManager
  exec/            # Executor, ExecutorConfig, air_exec, and the exec error
  exec_event.rs    # Core exec event pump loop
  model/           # Db, CRUD support, ModelManager, bus, and the BMCs
  model_change.rs  # Core model change pump loop
  model_rpc.rs     # model RPC handler
  prompts/         # system prompt composition
```

`lib.rs` registers the crate surface and re-exports the configuration and database handles:

```rust
// region:    --- Modules

pub mod config;
pub mod exec;
mod exec_event;
pub mod model;
mod model_change;
mod model_rpc;
mod prompts;

// endregion: --- Modules

pub use config::{Config, ConfigManager};
pub use model::Db;
```

## Configuration

- `config/` owns `Config`, `ConfigInner`, and `ConfigManager`.

- `Config` is a cheap, cloneable handle over `Arc<ConfigInner>` with builder style `with_*` and `append_*` helpers.

- `ConfigManager` loads `.zcoder/config.toml`, supports `refresh_if_modified()` hot reload, and exposes `get_config()`.

- The default config defines `[workspace] working_dir`, `[maestro] model`, `[model_sizes]`, and `[model_aliases]`.

- `get_model(ref_name)` resolves size presets such as `$small`, alias chains, and reasoning suffixes such as `-low`, `-high`, and `-max`, with cycle detection.

## Database and Model

- `model/` owns the SQLite layer: `Db` and `DbTx`, the private connection, and the schema (`recreate_db`, `create_schema`).

- The generic CRUD support (`crud_fns`, `db_bmc`, `prep_fields`) lives here.

- The `RunBmc` and `AirBmc` accessors and their `DbBmc` impls live here. The entity structs and their derives stay in `zc-core`; the accessors and the SQL live here.

- The model layer `Error` and `Result` live here, so the database failure identity is owned by the crate that can actually fail.

- `ModelManager` is a `OnceLock` singleton providing the shared `Db`, exposed through `get_model_manager()`.

- The model bus publishes model change events, exposed through `get_model_bus()`.

- `trim()` deletes run rows and is designed to be called at the start of a run; `db_size()` reports the database size.

## Executor

- `exec/` owns `Executor`, `ExecutorConfig`, the provider call helper `exec_air_chat`, the `prep_air_*` helpers, and the exec error.

- `Executor::new(config)` creates the command and event channels, syncs workspace assets, loads config, builds the AIPROG registry and script engine, composes the system prompt, and builds the base `ChatRequest`.

- `Executor::start()` consumes `ExecCmd` values until the command channel closes.

- `ExecutorConfig` carries `wspace_dir`, an optional `base_dir`, and an optional explicit `model`.

- The executor imports the contract types (`ExecCmd`, `ExecEvent`, and the channel aliases) from `zc_core::exec` and the model layer from `crate::model`.

## Prompts

- `prompts/` owns the system prompt composition.

- `maestro_entry_system(script_engine)` generates the system prompt.

- The prompt combines the UDIFFX file-change instructions with the generated AIPROG Lua API documentation.

- The generated system prompt is cached for inspection.

## Execution Pipeline

The `RunPrompt` path spans the TUI, the router, the executor, and the model layer.

1. The user presses `Enter` in the prompt view, and the TUI sends an exec command through the router.

2. `handle_run_prompt` creates a `Run` row and emits `ExecEvent::RunStart(run_id)`.

3. Workspace assets are re-synced and the config is hot reloaded before each run.

4. The model is resolved from the explicit model, or from `[maestro] model` through `get_model`, and the base directory is resolved from `--dir`, `[workspace] working_dir`, or `wspace_dir`.

5. The user prompt is appended to the base chat request, and `exec_air_chat` performs the provider call while recording an `Air` row with timing, tokens, and cost.

6. Raw request and response payloads are cached to `.zcoder` cache files for inspection.

7. The response text is parsed for UDIFFX file changes, which are extracted and applied to the base directory.

8. The remaining text is parsed for `<AIPROG>` Lua scripts, which run through the AIPROG script engine with a directory context scoped to the base directory.

9. Script results and remaining text are combined into the final answer.

10. The `Run` row is updated with the answer and end state, and `ExecEvent::RunEnd(run_id)` is emitted.

11. On failure, the `Run` row is updated with the error and `ExecEvent::RunError(run_id)` is emitted.

12. Model change events emitted during the run drive the TUI work info display, such as model name, elapsed time, tokens, and cost.

## Core Boundary Services

- `model_rpc.rs` owns `run_model_rpc_handler`, which serves `ModelRpcCmd` reads from the model layer and replies with `ModelRpcReply`, wrapping failures in `ModelRpcError`.

- `model_change.rs` owns the model change pump loop that forwards model bus events to the frontends.

- `exec_event.rs` owns the exec event pump loop that forwards executor lifecycle events to the frontends.

## Startup and Lifecycle

- `ZcBaseConfig` carries `wspace_dir`, an optional `base_dir`, and an optional explicit `model`, and converts into `ExecutorConfig`.

- `start_base_core(config)` starts Core initialization and the router dispatch loop: it creates the executor and spawns `executor.start()`, starts the model RPC handler, creates the router message channel, and spawns `run_router`.

- `InProcBase` is the temporary in-process stand-in for the future `zc base` server. It owns what the base role owns (Core initialization and the router loop), returns `router_msg_tx()` for a frontend, and `into_event_rx()` for the model change and exec event receivers.
- `InProcBase` builds the in-process `RouterClient` via `router_client()`, bundling the router message sender and the Core notification receivers into a single frontend handle.

- `ZcBase` is the future server. It starts the same base role and returns `router_msg_tx()` and `exec_event_rx()`.

- Both entry points must be called from within a Tokio runtime.

## Error Ownership

`zc-base::exec::Error` covers executor, config, prompt, provider, filesystem, and file-change application failures. External errors that occur inside execution behavior are converted into `zc-base` errors inside `zc-base`.

The model layer error covers database and CRUD failures.

`zc-base` converts the errors from the dependencies it directly uses. The root binary only converts the errors from the crate entry points it calls.

## Design Considerations

- The database and the executor live in the same crate on purpose. If the database moved to `zc-base` while the executor stayed in `zc-core`, then `zc-core` would have to call `zc-base` for `RunBmc::create` and `AirBmc::create_next`, creating a `zc-core` -> `zc-base` cycle. Keeping both here is what allows `zc-core` to stay types and contracts only.

- Owning the implementation in one crate makes the compiler enforce the boundary: removing the exports from `zc-core` means any crate that tries to reach SQLite fails to build.

- Keeping the message contract in `zc-router` and the implementation here lets the later process split be a transport swap. `InProcBase` and `ZcBase` start the same base role, so the split removes code from one place instead of untangling the TUI.

- `zc-base` depends on `zc-core` for types, `zc-router` for the contract, `zc-common` for shared helpers, and `zc-asset` to materialize the `.zcoder` workspace.
