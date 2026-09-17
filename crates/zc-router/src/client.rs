// region:    --- Modules

use crate::error::Result;
use crate::exec_event::ExecEventRx;
use crate::model_change::ModelChangeRx;
use crate::model_rpc::{ModelRpcError, ModelRpcReply, ModelRpcResult};
use crate::msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use zc_common::MsgId;
use zc_common::event_base::OnceTx;

// endregion: --- Modules

// region:    --- Types

/// Frontend client handle for sending messages and receiving notifications.
#[derive(Clone)]
pub struct RouterClient {
	inner: Arc<RouterClientInner>,
}

pub(crate) struct RouterClientInner {
	pub(crate) router_msg_tx: RouterMsgTx,
	pub(crate) model_change_rx: Mutex<Option<ModelChangeRx>>,
	pub(crate) exec_event_rx: Mutex<Option<ExecEventRx>>,
	pub(crate) pending_replies: Mutex<Option<PendingReplyMap>>,
}

type PendingReplyMap = HashMap<MsgId, OnceTx<ModelRpcReply>>;

impl std::fmt::Debug for RouterClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RouterClient").finish_non_exhaustive()
	}
}

// endregion: --- Types

// region:    --- Constructors & Transport

impl RouterClient {
	/// Creates an in-process router client from the send half and the two event receivers.
	pub fn in_proc(
		router_msg_tx: RouterMsgTx,
		model_change_rx: ModelChangeRx,
		exec_event_rx: ExecEventRx,
		reply_rx: RouterMsgRx,
	) -> Self {
		let client = Self {
			inner: Arc::new(RouterClientInner {
				router_msg_tx,
				model_change_rx: Mutex::new(Some(model_change_rx)),
				exec_event_rx: Mutex::new(Some(exec_event_rx)),
				pending_replies: Mutex::new(Some(HashMap::new())),
			}),
		};
		tokio::spawn(Self::run_reply_loop(Arc::downgrade(&client.inner), reply_rx));
		client
	}

	async fn run_reply_loop(inner: Weak<RouterClientInner>, mut reply_rx: RouterMsgRx) {
		while let Ok(msg) = reply_rx.recv().await {
			let Some(inner) = inner.upgrade() else {
				return;
			};
			match msg.data {
				RouterMsgData::ModelRpcRes(reply) => {
					Self { inner }.complete_pending(msg.msg_id, reply);
				}
				_ => tracing::warn!("unexpected message on model RPC reply channel"),
			}
		}
		if let Some(inner) = inner.upgrade() {
			let pending = inner
				.pending_replies
				.lock()
				.unwrap_or_else(|err| err.into_inner())
				.take();
			drop(pending);
		}
	}

	/// Sends a router message asynchronously.
	pub async fn send(&self, msg: RouterMsg) -> Result<()> {
		self.inner.router_msg_tx.send(msg).await.map_err(Into::into)
	}

	/// Returns a reference to the inner router message sender.
	pub fn router_msg_tx(&self) -> &RouterMsgTx {
		&self.inner.router_msg_tx
	}

	/// Takes the model change receiver if it has not been taken yet.
	pub fn take_model_change_rx(&self) -> Option<ModelChangeRx> {
		self.inner.model_change_rx.lock().ok().and_then(|mut guard| guard.take())
	}

	/// Takes the exec event receiver if it has not been taken yet.
	pub fn take_exec_event_rx(&self) -> Option<ExecEventRx> {
		self.inner.exec_event_rx.lock().ok().and_then(|mut guard| guard.take())
	}

	/// Returns the model change receiver if available.
	pub fn model_change_rx(&self) -> Option<ModelChangeRx> {
		self.take_model_change_rx()
	}

	/// Returns the exec event receiver if available.
	pub fn exec_event_rx(&self) -> Option<ExecEventRx> {
		self.take_exec_event_rx()
	}

	// region:    --- Pending Map

	pub(crate) fn register_pending(&self, msg_id: MsgId, res_tx: OnceTx<ModelRpcReply>) -> ModelRpcResult<()> {
		let mut pending = self.inner.pending_replies.lock().unwrap_or_else(|err| err.into_inner());
		let map = pending
			.as_mut()
			.ok_or_else(|| ModelRpcError::custom("model RPC reply channel is closed"))?;
		match map.entry(msg_id) {
			std::collections::hash_map::Entry::Vacant(entry) => {
				entry.insert(res_tx);
				Ok(())
			}
			std::collections::hash_map::Entry::Occupied(_) => {
				Err(ModelRpcError::custom("duplicate pending model RPC message id"))
			}
		}
	}

	pub(crate) fn remove_pending(&self, msg_id: MsgId) -> Option<OnceTx<ModelRpcReply>> {
		self.inner
			.pending_replies
			.lock()
			.unwrap_or_else(|err| err.into_inner())
			.as_mut()
			.and_then(|map| map.remove(&msg_id))
	}

	pub fn complete_pending(&self, msg_id: MsgId, reply: ModelRpcReply) {
		if let Some(res_tx) = self.remove_pending(msg_id) {
			res_tx.send(reply);
		}
	}

	// endregion: --- Pending Map
}

// endregion: --- Constructors & Transport

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::exec::ExecCmd;
	use crate::exec_event::new_exec_event_channel;
	use crate::model_change::new_model_change_channel;
	use crate::msg::new_router_msg_channel;
	use zc_common::event_base::new_once;

	#[tokio::test]
	async fn test_router_client_in_proc_lifecycle() -> Result<()> {
		let (router_tx, mut router_rx) = new_router_msg_channel();
		let (model_tx, model_rx) = new_model_change_channel();
		let (exec_tx, exec_rx) = new_exec_event_channel();
		let (reply_tx, reply_rx) = new_router_msg_channel();

		let client = RouterClient::in_proc(router_tx, model_rx, exec_rx, reply_rx);

		// -- Receivers take once
		let taken_model = client.take_model_change_rx();
		assert!(taken_model.is_some());
		assert!(client.take_model_change_rx().is_none());

		let taken_exec = client.take_exec_event_rx();
		assert!(taken_exec.is_some());
		assert!(client.take_exec_event_rx().is_none());

		// -- Send through client
		let msg = RouterMsg::new(RouterMsgData::Exec(ExecCmd::RunPrompt("test prompt".to_string())));
		client.send(msg).await?;

		let received = router_rx.recv().await?;
		assert!(matches!(received.data, RouterMsgData::Exec(ExecCmd::RunPrompt(_))));

		// -- Pending replies completion
		let (res_tx, res_rx) = new_once("test_client_pending");
		let test_msg_id = MsgId::new(999);
		client.register_pending(test_msg_id, res_tx)?;
		reply_tx
			.send(RouterMsg {
				msg_id: test_msg_id,
				wks_id: Default::default(),
				data: RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(1234))),
			})
			.await?;

		let reply = res_rx.recv().await?;
		match reply {
			ModelRpcReply::DbSize(Ok(size)) => assert_eq!(size, 1234),
			_ => panic!("unexpected reply"),
		}

		// Silence unused sender warnings in test
		drop(model_tx);
		drop(exec_tx);

		Ok(())
	}

	#[tokio::test]
	async fn test_router_client_pending_maps_are_isolated() -> Result<()> {
		let (router_tx_a, _router_rx_a) = new_router_msg_channel();
		let (_model_tx_a, model_rx_a) = new_model_change_channel();
		let (_exec_tx_a, exec_rx_a) = new_exec_event_channel();
		let (reply_tx_a, reply_rx_a) = new_router_msg_channel();
		let client_a = RouterClient::in_proc(router_tx_a, model_rx_a, exec_rx_a, reply_rx_a);

		let (router_tx_b, _router_rx_b) = new_router_msg_channel();
		let (_model_tx_b, model_rx_b) = new_model_change_channel();
		let (_exec_tx_b, exec_rx_b) = new_exec_event_channel();
		let (reply_tx_b, reply_rx_b) = new_router_msg_channel();
		let client_b = RouterClient::in_proc(router_tx_b, model_rx_b, exec_rx_b, reply_rx_b);

		let msg_id = MsgId::new(1);
		let (tx_a, rx_a) = new_once("isolated_reply_a");
		let (tx_b, rx_b) = new_once("isolated_reply_b");
		client_a.register_pending(msg_id, tx_a)?;
		client_b.register_pending(msg_id, tx_b)?;

		reply_tx_a
			.send(RouterMsg {
				msg_id,
				wks_id: Default::default(),
				data: RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(10))),
			})
			.await?;
		assert!(matches!(rx_a.recv().await?, ModelRpcReply::DbSize(Ok(10))));
		assert!(client_b.inner.pending_replies.lock().unwrap().as_ref().unwrap().contains_key(&msg_id));

		reply_tx_b
			.send(RouterMsg {
				msg_id,
				wks_id: Default::default(),
				data: RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(20))),
			})
			.await?;
		assert!(matches!(rx_b.recv().await?, ModelRpcReply::DbSize(Ok(20))));
		Ok(())
	}

	#[tokio::test]
	async fn test_router_client_rpc_return_path_and_disconnect() -> Result<()> {
		let (router_tx, mut router_rx) = new_router_msg_channel();
		let (_model_tx, model_rx) = new_model_change_channel();
		let (_exec_tx, exec_rx) = new_exec_event_channel();
		let (reply_tx, reply_rx) = new_router_msg_channel();
		let client = RouterClient::in_proc(router_tx, model_rx, exec_rx, reply_rx);

		let request_client = client.clone();
		let request = tokio::spawn(async move { crate::model_rpc::db_size(&request_client).await });
		let msg = router_rx.recv().await?;
		assert!(matches!(msg.data, RouterMsgData::ModelRpcReq(crate::model_rpc::ModelRpcReq::DbSize)));
		reply_tx
			.send(RouterMsg {
				msg_id: msg.msg_id,
				wks_id: msg.wks_id,
				data: RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(42))),
			})
			.await?;
		assert_eq!(request.await??, 42);

		let request_client = client.clone();
		let request = tokio::spawn(async move { crate::model_rpc::db_size(&request_client).await });
		router_rx.recv().await?;
		drop(reply_tx);
		assert!(request.await?.is_err());
		assert!(crate::model_rpc::db_size(&client).await.is_err());
		assert!(client.inner.pending_replies.lock().unwrap().is_none());
		Ok(())
	}

	#[tokio::test]
	async fn test_router_client_cancelled_rpc_removes_pending() -> Result<()> {
		let (router_tx, mut router_rx) = new_router_msg_channel();
		let (_model_tx, model_rx) = new_model_change_channel();
		let (_exec_tx, exec_rx) = new_exec_event_channel();
		let (_reply_tx, reply_rx) = new_router_msg_channel();
		let client = RouterClient::in_proc(router_tx, model_rx, exec_rx, reply_rx);

		let request_client = client.clone();
		let request = tokio::spawn(async move { crate::model_rpc::db_size(&request_client).await });
		router_rx.recv().await?;
		request.abort();
		assert!(request.await.unwrap_err().is_cancelled());
		assert!(client.inner.pending_replies.lock().unwrap().as_ref().unwrap().is_empty());
		Ok(())
	}
}

// endregion: --- Tests
