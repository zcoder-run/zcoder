# zc-router Specification

## Intent

Define the Core message contract and the routing layer between the frontends (the TUI, and the future base client) and the `zc-base` implementation.

`zc-router` owns the message envelope, the model RPC contract, and the router loop. It is contract and routing only: it never opens a database connection, never runs SQL, and never reads the model bus.

## Module Layout

```text
crates/zc-router/src/
  lib.rs          # module registry and public re-exports
  error.rs        # local Error and Result
  msg.rs          # RouterMsg, RouterMsgData, and the message channel
  exec.rs         # ExecCmd contract
  exec_event.rs   # ExecEvent contract and its channel
  model_change.rs # ModelChangeEvent contract and its channel
  model_rpc.rs    # ModelRpcCmd, ModelRpcReply, ModelRpcError, and the client facade
  router.rs       # run_router and route
```

`lib.rs` registers and re-exports the modules:

```rust
// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub mod exec;
pub mod exec_event;
pub mod model_change;
pub mod model_rpc;
pub mod msg;
pub mod router;

pub use exec::ExecCmd;
pub use exec_event::{ExecEvent, new_exec_event_channel};
pub use model_change::{
	ModelChangeEvent, ModelChangeRx, ModelChangeTx, new_model_change_channel,
};
pub use model_rpc::{
	ModelRpcCmd, ModelRpcCmdRx, ModelRpcCmdTx, ModelRpcError, ModelRpcReply, ModelRpcResult, air_get, air_list,
	db_size, new_model_rpc_cmd_channel, run_get, run_list,
};
pub use msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx, new_router_msg_channel};
pub use router::{route, run_router};

// endregion: --- Modules
```

## Message Envelope

`msg.rs` owns the envelope that carries every message across the boundary.

```rust
pub struct RouterMsg {
	pub msg_id: MsgId,
	pub wks_id: Id,
	pub data: RouterMsgData,
}

pub enum RouterMsgData {
	ModelRpc(ModelRpcCmd),
	ModelChange(ModelChangeEvent),
	Exec(ExecCmd),
	ExecEvent(ExecEvent),
}
```

- `msg_id` is a monotonic `MsgId` from the process-local source in `msg.rs`, so a message can be correlated without a shared clock.

- `wks_id` identifies the workspace the message belongs to. The single-workspace topology uses `Id::default()` for now.

- `RouterMsgData` is intentionally a mixed envelope: it carries commands toward Core and notifications coming back from it.

`RouterMsgTx`/`RouterMsgRx` alias the bounded mpsc channel, and `new_router_msg_channel()` creates the pair.

## The Router Loop

`router.rs` owns the dispatch loop.

```rust
pub async fn run_router(
	mut router_rx: RouterMsgRx,
	exec_cmd_tx: ExecCmdTx,
	model_rpc_cmd_tx: ModelRpcCmdTx,
) -> Result<()>
```

`route` dispatches one message:

- `RouterMsgData::Exec(cmd)` forwards to the executor command channel.

- `RouterMsgData::ModelRpc(cmd)` forwards to the `ModelRpcCmd` channel, which the `zc-base` model RPC handler serves.

- `RouterMsgData::ModelChange(event)` and `RouterMsgData::ExecEvent(event)` are logged at the router boundary.

Each route helper logs through `tracing::debug!` with the `->>` prefix and the `msg_id`/`wks_id` context.

## Model RPC Contract

`model_rpc.rs` owns the request and reply contract:

```rust
pub enum ModelRpcCmd {
	RunGet { id: Id, res_tx: OnceTx<ModelRpcReply> },
	RunList { options: ListRunOptions, res_tx: OnceTx<ModelRpcReply> },
	AirGet { id: Id, res_tx: OnceTx<ModelRpcReply> },
	AirList { options: ListAirOptions, res_tx: OnceTx<ModelRpcReply> },
	DbSize { res_tx: OnceTx<ModelRpcReply> },
}

pub enum ModelRpcReply {
	Run(ModelRpcResult<Option<Run>>),
	RunList(ModelRpcResult<Vec<Run>>),
	Air(ModelRpcResult<Option<Air>>),
	AirList(ModelRpcResult<Vec<Air>>),
	DbSize(ModelRpcResult<i64>),
}
```

- Each command carries a single-use `res_tx` reply handle. The router forwards the command, the `zc-base` handler performs the read and completes the handle, and the caller awaits the matching `res_rx` with no reverse channel and no correlation map.

- Errors travel inside the reply variant as `ModelRpcError`, so a failed read surfaces its cause instead of only a dropped channel.

The client facade hides the request/reply dance and returns plain results:

- `run_get`, `run_list`, `air_get`, `air_list`, `db_size`.

`ModelRpcCmdTx`/`ModelRpcCmdRx` alias the bounded mpsc channel created by `new_model_rpc_cmd_channel()`.

## Boundary Rules

- `zc-router` never references `RunBmc`, `AirBmc`, `ModelManager`, `get_model_manager`, or `get_model_bus`.

- `zc-router` never opens a `rusqlite::Connection`, runs SQL, or manages a transaction.

- `zc-router` forwards model RPC commands and returns replies; the read itself is owned by the `zc-base` handler.

- `zc-router` -> `zc-core` is for types and contracts only.

## Error Ownership

`zc-router::Error` is local to the crate and covers routing failures, such as a closed command channel.

`ModelRpcError` is the reply-level error. It is built by the `zc-base` handler from the model layer error and carried inside the reply variant, so the router never converts a model error type.

## Design Considerations

- Routing lives outside both the frontend and Core, so the frontend never holds a Core handle and Core never holds a frontend handle. Everything crosses as an owned message.

- Keeping the router free of the database and the model bus is what makes the later process split a transport swap rather than a rewrite: the same envelope, the same RPC contract, and the same client facade can move onto a wire transport.

- Carrying the reply handle inside the command keeps the async request/reply shape out of the frontend, so the TUI reads persisted state as plain owned data (`Run`, `Air`) instead of a live handle.

- The `res_tx` handle is a live in-process value, so this form stays behind the client facade, which is the seam where a serialization-friendly correlated reply can replace it at the wire boundary.
