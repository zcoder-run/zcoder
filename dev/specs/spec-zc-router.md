# zc-router Specification

## Intent

Define the Core message contract and the routing layer between the frontends (the TUI, and the future base client) and the `zc-base` implementation.

`zc-router` owns the message envelope, the model RPC contract, and the router loop. It is contract and routing only: it never opens a database connection, never runs SQL, and never reads the model bus.

## Module Layout

```text
crates/zc-router/src/
  lib.rs          # module registry and public re-exports
  error.rs        # local Error and Result
  client.rs       # RouterClient frontend handle and correlation map
  client_filter.rs# client_filter fan-out policy seam
  client_info.rs  # ClientInfo metadata for attach handshake
  msg.rs          # RouterMsg, RouterMsgData, and the message channel
  exec.rs         # ExecCmd contract
  exec_event.rs   # ExecEvent contract and its channel
  model_change.rs # ModelChangeEvent contract and its channel
  model_rpc.rs    # ModelRpcCmd, ModelRpcReq, ModelRpcReply, ModelRpcError, and the client facade
  router.rs       # run_router and route
  server.rs       # RouterServer, ConnWatch, and connection registry (server feature)
  transport/      # socket_path policy, wire framing, client_conn
  wspace_resolver.rs # WksResolver trait for workspace identity
```

`lib.rs` registers and re-exports the modules:

```rust
// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub mod client;
pub mod client_filter;
pub mod client_info;
pub mod exec;
pub mod exec_event;
pub mod model_change;
pub mod model_rpc;
pub mod msg;
pub mod router;
#[cfg(feature = "server")]
pub mod server;
pub mod transport;
pub mod wspace_resolver;

pub use client::RouterClient;
pub use client_filter::client_filter;
pub use client_info::ClientInfo;
pub use exec::ExecCmd;
pub use exec_event::{ExecEvent, ExecEventRx, ExecEventTx, new_exec_event_channel};
pub use model_change::{
	ModelChangeEvent, ModelChangeRx, ModelChangeTx, new_model_change_channel,
};
pub use model_rpc::{
	ModelRpcCmd, ModelRpcCmdRx, ModelRpcCmdTx, ModelRpcError, ModelRpcReply, ModelRpcReq, ModelRpcResult, air_get,
	air_list, db_size, new_model_rpc_cmd_channel, run_get, run_list,
};
pub use msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx, new_router_msg_channel};
pub use router::{route, run_router};
#[cfg(feature = "server")]
pub use server::{ConnWatch, RouterServer};
pub use wspace_resolver::WksResolver;

// endregion: --- Modules
```

## Message Envelope

`msg.rs` owns the envelope that carries every message across the boundary.

```rust
pub struct RouterMsg {
	pub msg_id: MsgId,
	pub wspace_id: Id,
	pub data: RouterMsgData,
}

pub enum RouterMsgData {
	ModelRpcReq(ModelRpcReq),
	ModelRpcRes(ModelRpcReply),
	ModelChange(ModelChangeEvent),
	Exec(ExecCmd),
	ExecEvent(ExecEvent),
	Attach(ClientInfo),
	AttachOk(Id),
	AttachErr(String),
}
```

- `msg_id` is a monotonic `MsgId` from the process-local source in `msg.rs`, so a message can be correlated without a shared clock.

- `wspace_id` identifies the workspace the message belongs to, assigned by the server at attach time and stamped onto outgoing messages by `RouterClient::send`.

- `RouterMsgData` is intentionally a mixed envelope: it carries commands toward Core and notifications coming back from it.

`RouterMsgTx`/`RouterMsgRx` alias the bounded mpsc channel, and `new_router_msg_channel()` creates the pair.

## Frontend Client Handle (RouterClient)

`client.rs` defines `RouterClient`, the single frontend-facing handle that encapsulates sending messages and receiving notifications.

```rust
pub struct RouterClient { ... }

impl RouterClient {
	pub fn in_proc(router_msg_tx: RouterMsgTx, model_change_rx: ModelChangeRx, exec_event_rx: ExecEventRx, reply_rx: RouterMsgRx) -> Self;
	#[cfg(feature = "client")]
	pub async fn uds(socket_path: impl AsRef<Path>, client_info: ClientInfo) -> Result<Self>;
	pub async fn send(&self, msg: RouterMsg) -> Result<()>;
	pub fn take_model_change_rx(&self) -> Option<ModelChangeRx>;
	pub fn take_exec_event_rx(&self) -> Option<ExecEventRx>;
	pub fn register_pending(&self, msg_id: MsgId, res_tx: OnceTx<ModelRpcReply>);
	pub fn complete_pending(&self, msg_id: MsgId, reply: ModelRpcReply);
	pub fn wspace_id(&self) -> Option<Id>;
}
```

- Frontends interact exclusively with `RouterClient`, keeping the concrete transport hidden behind its methods.

- `RouterClient` owns the correlation map `msg_id -> OnceTx<ModelRpcReply>`. Future wire transports will also manage request timeout and pending map sweep on connection loss inside `RouterClient`.

- When initialized over UDS via `RouterClient::uds`, the client executes an inline attach handshake before starting the reader task:
  1. Sends `RouterMsgData::Attach(client_info)`.
  2. Awaits `RouterMsgData::AttachOk(wspace_id)` or `RouterMsgData::AttachErr(err)`.
  3. On success, records `wspace_id` in internal client state.
  4. In `send(msg)`, automatically stamps `wspace_id` onto outgoing messages if `msg.wspace_id == Id::default()`.

- The `client` and `server` cargo features isolate the client implementation from server listeners.

## Router Server (Server Feature)

`server.rs` provides `RouterServer` for accepting inbound client connections over a Unix Domain Socket:

- **Initialization**: `RouterServer::bind(socket_path, exec_cmd_tx, model_rpc_cmd_tx, wspace_resolver, model_change_rx, exec_event_rx)` verifies that no live server is running, unlinks stale socket files, and binds the socket.
- **Per-Connection Handling**: Each client connection spawns an independent task:
  - Wraps split read and write stream halves with `WireReader` and `WireWriter`.
  - Performs the initial attach exchange: reads `Attach(client_info)`, queries `WksResolver::resolve(&client_info)`, and replies with `AttachOk(wspace_id)`.
  - Spawns a dedicated single-writer task draining a bounded per-connection response channel to prevent interleaved frame writes.
  - Dispatches regular messages to the shared `route` function.
- **Connection Registry and Fan-Out**:
  - Base-originated `ModelChangeEvent` and `ExecEvent` notifications fan out to all attached clients that pass `client_filter(client_wspace_id, msg)`.
  - `client_filter` passes events whose `msg.wspace_id` matches the client's `wspace_id`, as well as unscoped events carrying default `wspace_id`.
- **ConnWatch**: Exposes connection tracking (`wait_for_zero`, `wait_for_nonzero`, `count`) allowing the hosting server process to manage idle shutdown grace periods.

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

- `RouterMsgData::Exec(cmd)` wraps `cmd` with `msg.wspace_id` into `ExecReq` and forwards to the executor command channel.

- `RouterMsgData::ModelRpcReq(req)` converts into a local `ModelRpcCmd` with a local reply handle and forwards to the `ModelRpcCmd` channel. When the reply resolves, it completes the correlated pending response.

- `RouterMsgData::ModelRpcRes(reply)` completes a matching pending reply in the client correlation map.

- `RouterMsgData::ModelChange(event)` and `RouterMsgData::ExecEvent(event)` are logged at the router boundary.

Each route helper logs through `tracing::debug!` with the `->>` prefix and the `msg_id`/`wspace_id` context.

## Model RPC Contract

`model_rpc.rs` owns the request and reply contract:

```rust
pub enum ModelRpcReq {
	RunGet { id: Id },
	RunList { options: ListRunOptions },
	AirGet { id: Id },
	AirList { options: ListAirOptions },
	DbSize,
}

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

- `ModelRpcReq` is handle-free and serializable, crossing the router boundary inside `RouterMsgData::ModelRpcReq`.

- Correlation uses `msg_id` from the outer `RouterMsg`. The client facade registers a single-use `OnceTx` handle in `RouterClient`'s pending map and awaits the corresponding `OnceRx`.

- The router couples the incoming `ModelRpcReq` with a local `res_tx` handle to create `ModelRpcCmd` for `zc-base`, keeping handler execution decoupled from network boundaries.

- Errors travel inside the reply variant as `ModelRpcError`, so a failed read surfaces its cause instead of only a dropped channel.

The client facade hides the request/reply dance and returns plain results:

- `run_get(client, id)`, `run_list(client, options)`, `air_get(client, id)`, `air_list(client, options)`, `db_size(client)`.

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

- Keeping handle-free serializable requests (`ModelRpcReq`) and correlating replies via `msg_id` ensures the contract is network-ready without altering the facade interface exposed to frontends.

- `RouterClient` unifies outbound requests and inbound notification receivers into a single object, shielding frontends from underlying transport topologies.
