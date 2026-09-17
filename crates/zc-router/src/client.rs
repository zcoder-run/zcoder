// region:    --- Modules

use crate::error::Result;
use crate::exec_event::ExecEventRx;
use crate::model_change::ModelChangeRx;
use crate::model_rpc::ModelRpcReply;
use crate::msg::{RouterMsg, RouterMsgTx};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
	pub(crate) pending_replies: Arc<Mutex<HashMap<MsgId, OnceTx<ModelRpcReply>>>>,
}

impl std::fmt::Debug for RouterClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RouterClient").finish_non_exhaustive()
	}
}

// endregion: --- Types

// region:    --- Constructors & Transport

impl RouterClient {
	/// Creates an in-process router client from the send half and the two event receivers.
	pub fn in_proc(router_msg_tx: RouterMsgTx, model_change_rx: ModelChangeRx, exec_event_rx: ExecEventRx) -> Self {
		Self {
			inner: Arc::new(RouterClientInner {
				router_msg_tx,
				model_change_rx: Mutex::new(Some(model_change_rx)),
				exec_event_rx: Mutex::new(Some(exec_event_rx)),
				pending_replies: crate::model_rpc::in_proc_pending_map(),
			}),
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

	pub(crate) fn register_pending(&self, msg_id: MsgId, res_tx: OnceTx<ModelRpcReply>) {
		if let Ok(mut map) = self.inner.pending_replies.lock() {
			map.insert(msg_id, res_tx);
		}
	}

	pub(crate) fn remove_pending(&self, msg_id: MsgId) -> Option<OnceTx<ModelRpcReply>> {
		self.inner.pending_replies.lock().ok().and_then(|mut map| map.remove(&msg_id))
	}

	pub fn complete_pending(&self, msg_id: MsgId, reply: ModelRpcReply) {
		if let Some(res_tx) = self.remove_pending(msg_id) {
			res_tx.send(reply);
		}
	}

	// endregion: --- Pending Map
}

impl From<RouterMsgTx> for RouterClient {
	fn from(router_msg_tx: RouterMsgTx) -> Self {
		Self {
			inner: Arc::new(RouterClientInner {
				router_msg_tx,
				model_change_rx: Mutex::new(None),
				exec_event_rx: Mutex::new(None),
				pending_replies: crate::model_rpc::in_proc_pending_map(),
			}),
		}
	}
}

impl From<&RouterMsgTx> for RouterClient {
	fn from(router_msg_tx: &RouterMsgTx) -> Self {
		Self::from(router_msg_tx.clone())
	}
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
	use crate::msg::{RouterMsgData, new_router_msg_channel};
	use zc_common::event_base::new_once;

	#[tokio::test]
	async fn test_router_client_in_proc_lifecycle() -> Result<()> {
		let (router_tx, mut router_rx) = new_router_msg_channel();
		let (model_tx, model_rx) = new_model_change_channel();
		let (exec_tx, exec_rx) = new_exec_event_channel();

		let client = RouterClient::in_proc(router_tx, model_rx, exec_rx);

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
		client.register_pending(test_msg_id, res_tx);
		client.complete_pending(test_msg_id, ModelRpcReply::DbSize(Ok(1234)));

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
}

// endregion: --- Tests
