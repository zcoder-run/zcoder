use crate::client::RouterClient;
use crate::msg::{RouterMsg, RouterMsgData};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use zc_common::MsgId;
use zc_common::event_base::{OnceRx, OnceTx, new_once};
use zc_core::model::{Air, Id, ListAirOptions, ListRunOptions, Run};

// region:    --- Types

/// Wire-safe RPC request variants without live channel handles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelRpcReq {
	RunGet {
		id: Id,
	},
	RunList {
		#[serde(with = "list_options_serde")]
		options: ListRunOptions,
	},
	AirGet {
		id: Id,
	},
	AirList {
		#[serde(with = "list_options_serde")]
		options: ListAirOptions,
	},
	DbSize,
}

impl ModelRpcReq {
	/// Couples this request with a local single-use reply handle for Core processing.
	pub fn into_cmd(self, res_tx: OnceTx<ModelRpcReply>) -> ModelRpcCmd {
		match self {
			ModelRpcReq::RunGet { id } => ModelRpcCmd::RunGet { id, res_tx },
			ModelRpcReq::RunList { options } => ModelRpcCmd::RunList { options, res_tx },
			ModelRpcReq::AirGet { id } => ModelRpcCmd::AirGet { id, res_tx },
			ModelRpcReq::AirList { options } => ModelRpcCmd::AirList { options, res_tx },
			ModelRpcReq::DbSize => ModelRpcCmd::DbSize { res_tx },
		}
	}
}

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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum ModelRpcReply {
	Run(ModelRpcResult<Option<Run>>),
	RunList(ModelRpcResult<Vec<Run>>),
	Air(ModelRpcResult<Option<Air>>),
	AirList(ModelRpcResult<Vec<Air>>),
	DbSize(ModelRpcResult<i64>),
}

/// Error carried by a [`ModelRpcReply`] variant.
#[derive(Debug, Clone, Display, Serialize, Deserialize)]
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

// region:    --- Pending Correlation Map

pub(crate) type PendingReplyMap = HashMap<MsgId, OnceTx<ModelRpcReply>>;

static IN_PROC_PENDING_MAP: LazyLock<Arc<Mutex<PendingReplyMap>>> =
	LazyLock::new(|| Arc::new(Mutex::new(HashMap::new())));

pub(crate) fn in_proc_pending_map() -> Arc<Mutex<PendingReplyMap>> {
	Arc::clone(&IN_PROC_PENDING_MAP)
}

pub(crate) fn complete_pending(msg_id: MsgId, reply: ModelRpcReply) {
	if let Ok(mut map) = IN_PROC_PENDING_MAP.lock()
		&& let Some(res_tx) = map.remove(&msg_id)
	{
		res_tx.send(reply);
	}
}

// endregion: --- Pending Correlation Map

// region:    --- Client Facade

/// Requests a run by id through the router client and awaits the single-use reply.
pub async fn run_get(client: &RouterClient, id: Id) -> ModelRpcResult<Option<Run>> {
	let (res_tx, res_rx) = new_once("model_rpc_run_get");
	let req = ModelRpcReq::RunGet { id };
	match request(client, req, res_tx, res_rx).await? {
		ModelRpcReply::Run(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for run_get")),
	}
}

/// Requests a run list through the router client and awaits the single-use reply.
pub async fn run_list(client: &RouterClient, options: ListRunOptions) -> ModelRpcResult<Vec<Run>> {
	let (res_tx, res_rx) = new_once("model_rpc_run_list");
	let req = ModelRpcReq::RunList { options };
	match request(client, req, res_tx, res_rx).await? {
		ModelRpcReply::RunList(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for run_list")),
	}
}

/// Requests an air by id through the router client and awaits the single-use reply.
pub async fn air_get(client: &RouterClient, id: Id) -> ModelRpcResult<Option<Air>> {
	let (res_tx, res_rx) = new_once("model_rpc_air_get");
	let req = ModelRpcReq::AirGet { id };
	match request(client, req, res_tx, res_rx).await? {
		ModelRpcReply::Air(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for air_get")),
	}
}

/// Requests an air list through the router client and awaits the single-use reply.
pub async fn air_list(client: &RouterClient, options: ListAirOptions) -> ModelRpcResult<Vec<Air>> {
	let (res_tx, res_rx) = new_once("model_rpc_air_list");
	let req = ModelRpcReq::AirList { options };
	match request(client, req, res_tx, res_rx).await? {
		ModelRpcReply::AirList(reply) => reply,
		_ => Err(ModelRpcError::custom("unexpected reply for air_list")),
	}
}

/// Requests the database size in bytes through the router client and awaits the single-use reply.
pub async fn db_size(client: &RouterClient) -> ModelRpcResult<i64> {
	let (res_tx, res_rx) = new_once("model_rpc_db_size");
	let req = ModelRpcReq::DbSize;
	match request(client, req, res_tx, res_rx).await? {
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

// region:    --- ListOptions Serde

mod list_options_serde {
	use super::ListRunOptions;
	use serde::{Deserialize, Deserializer, Serialize, Serializer};

	#[derive(Serialize, Deserialize)]
	struct ListOptionsDef {
		offset: Option<i64>,
		limit: Option<i64>,
		order_bys: Option<String>,
	}

	fn order_bys_to_string(ob_debug: &str) -> Option<String> {
		let mut parts = Vec::new();
		let mut rest = ob_debug;
		while let Some(start_quote) = rest.find('"') {
			let after_start = &rest[start_quote + 1..];
			let Some(end_quote) = after_start.find('"') else { break };
			let col = &after_start[..end_quote];
			let after_end = &after_start[end_quote + 1..];

			let next_delim = after_end.find(['}', ')', '"']).unwrap_or(after_end.len());
			let segment = &after_end[..next_delim];
			let dir = if segment.contains("Desc") || segment.contains("desc") {
				"desc"
			} else {
				"asc"
			};
			parts.push(format!("{col} {dir}"));
			rest = &after_end[next_delim..];
		}
		if parts.is_empty() { None } else { Some(parts.join(", ")) }
	}

	pub fn serialize<S>(options: &ListRunOptions, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let def = ListOptionsDef {
			offset: options.offset,
			limit: options.limit,
			order_bys: options
				.order_bys
				.as_ref()
				.and_then(|ob| order_bys_to_string(&format!("{ob:?}"))),
		};
		def.serialize(serializer)
	}

	pub fn deserialize<'de, D>(deserializer: D) -> Result<ListRunOptions, D::Error>
	where
		D: Deserializer<'de>,
	{
		let def = ListOptionsDef::deserialize(deserializer)?;
		let options = ListRunOptions {
			offset: def.offset,
			limit: def.limit,
			order_bys: def.order_bys.map(|order_bys| order_bys.as_str().into()),
		};
		Ok(options)
	}
}

// endregion: --- ListOptions Serde

// region:    --- Support

/// Sends a model RPC request and awaits its correlated reply.
async fn request(
	client: &RouterClient,
	req: ModelRpcReq,
	res_tx: OnceTx<ModelRpcReply>,
	res_rx: OnceRx<ModelRpcReply>,
) -> ModelRpcResult<ModelRpcReply> {
	let msg = RouterMsg::new(RouterMsgData::ModelRpcReq(req));
	let msg_id = msg.msg_id;
	client.register_pending(msg_id, res_tx);

	if let Err(err) = client.send(msg).await {
		client.remove_pending(msg_id);
		return Err(ModelRpcError::custom(err.to_string()));
	}
	res_rx.recv().await.map_err(|err| ModelRpcError::custom(err.to_string()))
}

// endregion: --- Support

// region:    --- Error Boilerplate

impl std::error::Error for ModelRpcError {}

// endregion: --- Error Boilerplate

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_model_rpc_req_serde_roundtrip() -> Result<()> {
		let req = ModelRpcReq::RunGet { id: Id::default() };
		let json = serde_json::to_string(&req)?;
		let back: ModelRpcReq = serde_json::from_str(&json)?;
		match back {
			ModelRpcReq::RunGet { id } => assert_eq!(id, Id::default()),
			_ => panic!("unexpected deserialized variant"),
		}

		let mut options = ListRunOptions::default().with_offset(10).with_limit(20);
		options.order_bys = Some("ctime desc".into());
		let req_list = ModelRpcReq::RunList { options };
		let json = serde_json::to_string(&req_list)?;
		let back_list: ModelRpcReq = serde_json::from_str(&json)?;
		match back_list {
			ModelRpcReq::RunList { options } => {
				assert_eq!(options.offset, Some(10));
				assert_eq!(options.limit, Some(20));
				assert!(options.order_bys.is_some());
			}
			_ => panic!("unexpected deserialized variant"),
		}

		Ok(())
	}

	#[test]
	fn test_model_rpc_reply_serde_roundtrip() -> Result<()> {
		// -- Setup & Fixtures
		let reply = ModelRpcReply::DbSize(Ok(42));

		// -- Exec
		let json = serde_json::to_string(&reply)?;
		let back: ModelRpcReply = serde_json::from_str(&json)?;

		// -- Check
		match back {
			ModelRpcReply::DbSize(Ok(val)) => assert_eq!(val, 42),
			_ => panic!("unexpected deserialized variant"),
		}

		Ok(())
	}

	#[test]
	fn test_model_rpc_error_serde_roundtrip() -> Result<()> {
		// -- Setup & Fixtures
		let err = ModelRpcError::custom("test error");

		// -- Exec
		let json = serde_json::to_string(&err)?;
		let back: ModelRpcError = serde_json::from_str(&json)?;

		// -- Check
		assert_eq!(back.message, "test error");

		Ok(())
	}
}

// endregion: --- Tests
