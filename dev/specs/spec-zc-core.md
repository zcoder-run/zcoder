# zc-core Specification

## Intent

Define the shared data types and event contracts that cross crate boundaries in the zcoder workspace.

`zc-core` is the types and contracts crate. It owns the models and the messages that other crates pass around, and it performs no execution and no persistence. The database and the execution engine are owned by `zc-base`.

`zc-core` must not depend on `zc-router` or `zc-base`.

## Module Layout

```text
crates/zc-core/src/
  lib.rs             # module registry and crate documentation
  derive_aliases.rs  # internal derive alias helpers
  exec/
    exec_event.rs    # ExecCmd, ExecReq, ExecEvent, and channel aliases
  model/
    types.rs         # Id, EpochUs, EntityType, EntityAction, RelIds
    entities/        # air, common, run, and wspace entity types with their derives
    bus/
      model_event.rs # ModelChangeEvent and its data
```

`lib.rs` registers and exposes the `exec` and `model` modules:

```rust
//! Core data types and event contracts for zcoder.
//!
//! This crate contains shared data structures, entity models, and event contracts.
//! It does not perform persistence or execution. The database and execution engine
//! are owned by `zc-base`.

// region:    --- Modules

mod derive_aliases;

use derive_aliases::*;

pub mod exec;
pub mod model;

// endregion: --- Modules
```

## exec

`exec` exposes only the event contract. The executor itself lives in `zc-base`.

```rust
pub enum ExecCmd {
	RunPrompt(String),
}

pub struct ExecReq {
	pub wspace_id: Id,
	pub cmd: ExecCmd,
}

pub type ExecReqRx = zc_common::event_base::MpscRx<ExecReq>;
pub type ExecReqTx = zc_common::event_base::MpscTx<ExecReq>;
pub type ExecCmdRx = ExecReqRx;
pub type ExecCmdTx = ExecReqTx;

pub enum ExecEvent {
	RunStart(Id),
	RunEnd(Id),
	RunError(Id),
}

pub type ExecEventRx = zc_common::event_base::MpscRx<ExecEvent>;
pub type ExecEventTx = zc_common::event_base::MpscTx<ExecEvent>;
```

`ExecCmd` carries frontend intent toward the router, which wraps it in `ExecReq` with the client's `wspace_id` toward the executor. `ExecEvent` carries run lifecycle notifications back toward the frontends. The channel aliases keep every producer and consumer on the same bounded mpsc contract.

## model

- `model/types.rs` owns the shared model types: `Id`, `EpochUs`, `EntityType` (including `EntityType::Wks`), `EntityAction`, and `RelIds` (which includes `wspace_id: Option<Id>`).

- `model/entities/` owns the entity struct types and their companions: `Run`, `RunForCreate`, `RunForUpdate`, `RunEndState`, `Air`, `AirForCreate`, `AirForUpdate`, `AirEndState`, `Wks`, `WksForCreate`, `ListRunOptions`, `ListAirOptions`, and `ListWksOptions`.

- `model/bus/model_event.rs` owns `ModelChangeEvent` and its data, the contract published when persisted entities change.

The entity structs keep their `modql` derives (`Fields`, `SqliteFromRow`) where the type is defined, because the generated impl must live with the type. A derive is compile-time codegen: it opens no connection and runs no SQL. This is the reason `zc-core` keeps a `modql` dependency, and the reason `zc-core` can describe persisted shapes without owning persistence.

## What zc-core Does Not Own

`zc-core` has no `Db`, no `DbTx`, no schema, no CRUD support, no `ModelManager`, no model bus, no `RunBmc`/`AirBmc` accessors, no config, no prompts, and no executor.

It never opens a `rusqlite::Connection`, runs SQL, or manages a transaction. All of that lives in `zc-base`.

## Error Ownership

`zc-core` defines no error type. It holds data and contracts only, so there is nothing for another crate to convert at this boundary. The `TryFrom<String>` impls on `Id` and `EpochUs` use `String` as their error type, which keeps the crate free of an error identity that the consumers would have to re-map.

Execution, database, config, and prompt failures are owned by `zc-base`.

## Design Considerations

- Keeping the types and the events in a dedicated crate lets the UI side and the base side share them without either side depending on the other.

- The `zc-core` -> `zc-base` direction must never appear. If the database and the executor live in `zc-base`, then `zc-core` calling `zc-base` for `RunBmc::create` or `AirBmc::create_next` would create a cycle, which is why neither the database nor the executor stays here.

- The entity derives stay in `zc-core` because Rust requires the generated impl where the type is defined. The cost is a `modql` dependency; the benefit is a single definition of each persisted shape.

- A types-only crate keeps the dependency graph explicit: everything that needs the shared models depends on `zc-core`, and nothing that only needs the models has to accept the database or the executor.
