use crate::msg::{RouterMsg, RouterMsgData, RouterMsgTx};
use derive_more::Display;
use zc_common::event_base::{OnceRx, OnceTx, new_once};
use zc_core::model::{Air, Id, ListAirOptions, ListRunOptions, Run};

// region:    --- Types

/// RPC-style commands directed at Core model operations.
///
/// Each command carries a single-use `res_tx` reply handle. The router completes
/// it after the read, so the caller awaits the matching `res_rx` without a
/// reverse channel or a correlation map. The handle is a live in-process value,
/// so this form stays behind the client facade below, which is the seam where a
/// serialization-friendly correlated reply can replace it at the wire boundary.
#[derive(Debug)]
pub enum ModelRpcCmd {
	// -- Run
	RunGet {
		id: Id,
		res_tx: OnceTx<ModelRpcReply>,
	},

	RunList {
		options: ListRunOptions,
		res_tx: OnceTx<ModelRpcReply>,
	},

	// -- Air
	AirGet {
		id: Id,
		res_tx: OnceTx<ModelRpcReply>,
	},

	AirList {
		options: ListAirOptions,
		res_tx: OnceTx<ModelRpcReply>,
	},

	// -- Db
	DbSize {
		res_tx: OnceTx<ModelRpcReply>,
	},
}

/// Reply payload returned for a [`ModelRpcCmd`].
///
/// Errors travel inside the variant, so a failed read surfaces its cause
/// instead of only a dropped channel.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ModelRpcReply {
	Run(ModelRpcResult<Option<Run>>),
	RunList(ModelRpcResult<Vec<Run>>),
	Air(ModelRpcResult<Option<Air>>),
	AirList(ModelRpcResult<Vec<Air>>),
	DbSize(ModelRpcResult<i64>),
}

/// Error carried by a [`ModelRpcReply`] variant.
#[derive(Debug, Display)]
#[display("{message}")]
pub struct ModelRpcError {
	message: String,
}

/// Result type for model RPC replies.
pub type ModelRpcResult<T> = core::result::Result<T, ModelRpcError>;

// endregion: --- Types

// region:    --- Model RPC Command Channel

/// Channel carrying model RPC commands from the router to the `zc-base` handler.
pub type ModelRpcCmdTx = zc_common::event_base::MpscTx<ModelRpcCmd>;
pub type ModelRpcCmdRx = zc_common::event_base::MpscRx<ModelRpcCmd>;

/// Creates the bounded `ModelRpcCmd` channel used to forward model RPC commands to a handler.
pub fn new_model_rpc_cmd_channel() -> (ModelRpcCmdTx, ModelRpcCmdRx) {
	let (tx, rx) = zc_common::event_base::new_mpsc_bounded_default("model_rpc_cmd_channel")
		.expect("model rpc cmd channel capacity is non-zero");
	(tx, rx)
}

// endregion: --- Model RPC Command Channel

// region:    --- Client Facade

/// Requests a run by id through the router and awaits the single-use reply.
pub async fn run_get(router_msg_tx: &RouterMsgTx, id: Id) -> ModelRpcResult<Option<Run>> {
	let (res_tx, res_rx) = new_once("model_rpc_run_get");
	let cmd = ModelRpcCmd::RunGet { id, res_tx };
	match request(router_msg_tx, cmd, res_rx).await? {
		ModelRpcReply::Run(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for run_get")),
	}
}

/// Requests a run list through the router and awaits the single-use reply.
pub async fn run_list(router_msg_tx: &RouterMsgTx, options: ListRunOptions) -> ModelRpcResult<Vec<Run>> {
	let (res_tx, res_rx) = new_once("model_rpc_run_list");
	let cmd = ModelRpcCmd::RunList { options, res_tx };
	match request(router_msg_tx, cmd, res_rx).await? {
		ModelRpcReply::RunList(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for run_list")),
	}
}

/// Requests an air by id through the router and awaits the single-use reply.
pub async fn air_get(router_msg_tx: &RouterMsgTx, id: Id) -> ModelRpcResult<Option<Air>> {
	let (res_tx, res_rx) = new_once("model_rpc_air_get");
	let cmd = ModelRpcCmd::AirGet { id, res_tx };
	match request(router_msg_tx, cmd, res_rx).await? {
		ModelRpcReply::Air(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for air_get")),
	}
}

/// Requests an air list through the router and awaits the single-use reply.
pub async fn air_list(router_msg_tx: &RouterMsgTx, options: ListAirOptions) -> ModelRpcResult<Vec<Air>> {
	let (res_tx, res_rx) = new_once("model_rpc_air_list");
	let cmd = ModelRpcCmd::AirList { options, res_tx };
	match request(router_msg_tx, cmd, res_rx).await? {
		ModelRpcReply::AirList(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for air_list")),
	}
}

/// Requests the database size in bytes through the router and awaits the single-use reply.
pub async fn db_size(router_msg_tx: &RouterMsgTx) -> ModelRpcResult<i64> {
	let (res_tx, res_rx) = new_once("model_rpc_db_size");
	let cmd = ModelRpcCmd::DbSize { res_tx };
	match request(router_msg_tx, cmd, res_rx).await? {
		ModelRpcReply::DbSize(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for db_size")),
	}
}

// endregion: --- Client Facade

// region:    --- Custom

impl ModelRpcError {
	pub fn custom(message: impl Into<String>) -> Self {
		Self {
			message: message.into(),
		}
	}
}

// endregion: --- Custom

// region:    --- Support

/// Sends a model RPC command and awaits its single-use reply.
async fn request(
	router_msg_tx: &RouterMsgTx,
	cmd: ModelRpcCmd,
	res_rx: OnceRx<ModelRpcReply>,
) -> ModelRpcResult<ModelRpcReply> {
	let msg = RouterMsg::new(RouterMsgData::ModelRpc(cmd));
	router_msg_tx
		.send(msg)
		.await
		.map_err(|err| ModelRpcError::custom(err.to_string()))?;
	res_rx.recv().await.map_err(|err| ModelRpcError::custom(err.to_string()))
}

// endregion: --- Support

// region:    --- Error Boilerplate

impl std::error::Error for ModelRpcError {}

// endregion: --- Error Boilerplate
