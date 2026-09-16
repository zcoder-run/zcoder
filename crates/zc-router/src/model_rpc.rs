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

// endregion: --- Client Facade

// region:    --- Froms

impl From<zc_core::model::Error> for ModelRpcError {
	fn from(err: zc_core::model::Error) -> Self {
		Self::custom(err.to_string())
	}
}

// endregion: --- Froms

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
