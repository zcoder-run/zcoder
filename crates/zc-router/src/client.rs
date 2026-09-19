#[cfg(feature = "client")]
use crate::client_info::ClientInfo;
#[cfg(feature = "client")]
use crate::error::Error;
use crate::error::Result;
use crate::exec_event::ExecEventRx;
use crate::model_change::ModelChangeRx;
use crate::model_rpc::{ModelRpcError, ModelRpcReply, ModelRpcResult};
use crate::msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx};
#[cfg(feature = "client")]
use crate::transport::{ClientConn, ClientConnSink, WireReader, WireWriter};
use std::collections::HashMap;
#[cfg(feature = "client")]
use std::path::Path;
use std::sync::{Arc, Mutex, Weak};
#[cfg(feature = "client")]
use tokio::net::UnixStream;
use zc_common::MsgId;
use zc_common::event_base::OnceTx;
use zc_core::model::Id;

// region:    --- Types

/// Frontend client handle for sending messages and receiving notifications.
#[derive(Clone)]
pub struct RouterClient {
	inner: Arc<RouterClientInner>,
}

pub(crate) struct RouterClientInner {
	pub(crate) outbound: Outbound,
	pub(crate) wspace_id: Mutex<Option<Id>>,
	pub(crate) model_change_rx: Mutex<Option<ModelChangeRx>>,
	pub(crate) exec_event_rx: Mutex<Option<ExecEventRx>>,
	pub(crate) pending_replies: Mutex<Option<PendingReplyMap>>,
}

pub(crate) enum Outbound {
	InProc(RouterMsgTx),
	#[cfg(feature = "client")]
	Uds(ClientConn),
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
				outbound: Outbound::InProc(router_msg_tx),
				wspace_id: Mutex::new(None),
				model_change_rx: Mutex::new(Some(model_change_rx)),
				exec_event_rx: Mutex::new(Some(exec_event_rx)),
				pending_replies: Mutex::new(Some(HashMap::new())),
			}),
		};
		tokio::spawn(Self::run_reply_loop(Arc::downgrade(&client.inner), reply_rx));
		client
	}

	/// Connects to a base server over a Unix domain socket, performs the attach
	/// handshake, and returns a client handle ready to send and receive.
	#[cfg(feature = "client")]
	pub async fn uds(socket_path: impl AsRef<Path>, client_info: ClientInfo) -> Result<Self> {
		let stream = UnixStream::connect(socket_path.as_ref()).await?;
		let (reader, writer) = stream.into_split();
		let mut reader = WireReader::<_, RouterMsg>::new(reader);
		let mut writer = WireWriter::<_, RouterMsg>::new(writer);

		// -- Inline attach handshake before the reader task starts.
		let attach_msg = RouterMsg::new(RouterMsgData::Attach(client_info));
		let attach_msg_id = attach_msg.msg_id;
		writer.write_frame(&attach_msg).await?;

		let reply = reader
			.read_frame()
			.await?
			.ok_or_else(|| Error::custom("connection closed during attach"))?;

		let wspace_id = match reply.data {
			RouterMsgData::AttachOk(id) if reply.msg_id == attach_msg_id => id,
			RouterMsgData::AttachErr(err) if reply.msg_id == attach_msg_id => return Err(Error::custom(err)),
			_ => return Err(Error::custom("unexpected response during attach")),
		};

		// -- Local event channels for the client facade.
		let (model_tx, model_rx) = crate::model_change::new_model_change_channel();
		let (exec_tx, exec_rx) = crate::exec_event::new_exec_event_channel();

		let (model_fwd_tx, mut model_fwd_rx) = tokio::sync::mpsc::unbounded_channel::<RouterMsg>();
		tokio::spawn(async move {
			while let Some(msg) = model_fwd_rx.recv().await {
				if model_tx.send(msg).await.is_err() {
					break;
				}
			}
		});

		let (exec_fwd_tx, mut exec_fwd_rx) = tokio::sync::mpsc::unbounded_channel::<RouterMsg>();
		tokio::spawn(async move {
			while let Some(msg) = exec_fwd_rx.recv().await {
				if exec_tx.send(msg).await.is_err() {
					break;
				}
			}
		});

		let inner = Arc::new_cyclic(|weak_inner| {
			let sink = Arc::new(UdsSink {
				inner: weak_inner.clone(),
				model_tx: model_fwd_tx,
				exec_tx: exec_fwd_tx,
			});
			let conn = ClientConn::from_halves("router_client_uds", reader, writer, sink);
			RouterClientInner {
				outbound: Outbound::Uds(conn),
				wspace_id: Mutex::new(Some(wspace_id)),
				model_change_rx: Mutex::new(Some(model_rx)),
				exec_event_rx: Mutex::new(Some(exec_rx)),
				pending_replies: Mutex::new(Some(HashMap::new())),
			}
		});

		Ok(Self { inner })
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
			let pending = inner.pending_replies.lock().unwrap_or_else(|err| err.into_inner()).take();
			drop(pending);
		}
	}

	/// Sends a router message asynchronously.
	pub async fn send(&self, mut msg: RouterMsg) -> Result<()> {
		if msg.wspace_id == Id::default()
			&& let Some(wspace_id) = self.wspace_id()
		{
			msg.wspace_id = wspace_id;
		}
		match &self.inner.outbound {
			Outbound::InProc(tx) => tx.send(msg).await.map_err(Into::into),
			#[cfg(feature = "client")]
			Outbound::Uds(conn) => conn.send(msg).await,
		}
	}

	/// Returns a reference to the inner router message sender if in-process.
	pub fn router_msg_tx(&self) -> Option<&RouterMsgTx> {
		match &self.inner.outbound {
			Outbound::InProc(tx) => Some(tx),
			#[cfg(feature = "client")]
			Outbound::Uds(_) => None,
		}
	}

	/// Returns the workspace id assigned to this client, if any.
	pub fn wspace_id(&self) -> Option<Id> {
		*self.inner.wspace_id.lock().unwrap_or_else(|err| err.into_inner())
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

#[cfg(feature = "client")]
struct UdsSink {
	inner: Weak<RouterClientInner>,
	model_tx: tokio::sync::mpsc::UnboundedSender<RouterMsg>,
	exec_tx: tokio::sync::mpsc::UnboundedSender<RouterMsg>,
}

#[cfg(feature = "client")]
impl ClientConnSink for UdsSink {
	fn on_reply(&self, msg_id: MsgId, reply: ModelRpcReply) {
		if let Some(inner) = self.inner.upgrade() {
			RouterClient { inner }.complete_pending(msg_id, reply);
		}
	}

	fn on_model_change(&self, msg: RouterMsg) {
		let _ = self.model_tx.send(msg);
	}

	fn on_exec_event(&self, msg: RouterMsg) {
		let _ = self.exec_tx.send(msg);
	}

	fn on_closed(&self) {
		if let Some(inner) = self.inner.upgrade() {
			let pending = inner.pending_replies.lock().unwrap_or_else(|err| err.into_inner()).take();
			drop(pending);
		}
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
				wspace_id: Default::default(),
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
				wspace_id: Default::default(),
				data: RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(10))),
			})
			.await?;
		assert!(matches!(rx_a.recv().await?, ModelRpcReply::DbSize(Ok(10))));
		assert!(
			client_b
				.inner
				.pending_replies
				.lock()
				.unwrap()
				.as_ref()
				.unwrap()
				.contains_key(&msg_id)
		);

		reply_tx_b
			.send(RouterMsg {
				msg_id,
				wspace_id: Default::default(),
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
		assert!(matches!(
			msg.data,
			RouterMsgData::ModelRpcReq(crate::model_rpc::ModelRpcReq::DbSize)
		));
		reply_tx
			.send(RouterMsg {
				msg_id: msg.msg_id,
				wspace_id: msg.wspace_id,
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

	#[cfg(all(feature = "client", feature = "server"))]
	mod uds_tests {
		type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

		use super::*;
		use crate::WksResolver;
		use crate::model_change::{ModelChangeEvent, ModelChangeTx, new_model_change_channel};
		use crate::model_rpc::{ModelRpcCmd, ModelRpcCmdRx, new_model_rpc_cmd_channel};
		use crate::server::{ConnWatch, RouterServer};
		use crate::transport::unlink_if_exists;
		use futures_util::future::BoxFuture;
		use std::path::PathBuf;
		use std::time::Duration;
		use zc_core::model::{EntityAction, EntityType, RelIds};

		struct StubResolver {
			id: Id,
			fail: bool,
		}

		impl WksResolver for StubResolver {
			fn resolve<'a>(&'a self, _info: &'a ClientInfo) -> BoxFuture<'a, crate::error::Result<Id>> {
				Box::pin(async move {
					if self.fail {
						Err(crate::error::Error::custom("stub resolver failure"))
					} else {
						Ok(self.id)
					}
				})
			}
		}

		struct PathStubResolver;

		impl WksResolver for PathStubResolver {
			fn resolve<'a>(&'a self, info: &'a ClientInfo) -> BoxFuture<'a, crate::error::Result<Id>> {
				Box::pin(async move {
					let id_str = if info.wspace_dir.ends_with("zc-a") {
						"00000000-0000-0000-0000-000000000001"
					} else {
						"00000000-0000-0000-0000-000000000002"
					};
					Ok(Id::try_from(id_str.to_string())?)
				})
			}
		}

		async fn start_server(
			socket_name: &str,
			resolver: Arc<dyn WksResolver>,
		) -> Result<(PathBuf, ConnWatch, ModelRpcCmdRx, ModelChangeTx)> {
			let socket_path = std::env::temp_dir().join(socket_name);
			unlink_if_exists(&socket_path)?;

			let (exec_cmd_tx, _exec_cmd_rx) = zc_common::event_base::new_mpsc_bounded("test_exec_cmd", 8)?;
			let (model_rpc_cmd_tx, model_rpc_cmd_rx) = new_model_rpc_cmd_channel();
			let (model_change_tx, model_change_rx) = new_model_change_channel();
			let (_exec_event_tx, exec_event_rx) = crate::exec_event::new_exec_event_channel();

			let server = RouterServer::bind(
				&socket_path,
				exec_cmd_tx,
				model_rpc_cmd_tx,
				resolver,
				model_change_rx,
				exec_event_rx,
			)
			.await?;
			let watch = server.conn_watch();
			tokio::spawn(async move {
				if let Err(err) = server.run().await {
					tracing::error!("->> test server stopped: {err}");
				}
			});

			Ok((socket_path, watch, model_rpc_cmd_rx, model_change_tx))
		}

		#[tokio::test]
		async fn test_router_client_uds_attach_and_rpc() -> Result<()> {
			let assigned_id = Id::try_from("00000000-0000-0000-0000-000000000077".to_string())?;
			let resolver = Arc::new(StubResolver {
				id: assigned_id,
				fail: false,
			});
			let (socket_path, _watch, mut rpc_rx, _model_tx) =
				start_server("zc-router-test-uds-attach-rpc.sock", resolver).await?;

			tokio::spawn(async move {
				if let Ok(ModelRpcCmd::DbSize { res_tx }) = rpc_rx.recv().await {
					res_tx.send(ModelRpcReply::DbSize(Ok(42)));
				}
			});

			let client = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-wspace")).await?;
			assert_eq!(client.wspace_id(), Some(assigned_id));

			let size = crate::model_rpc::db_size(&client).await?;
			assert_eq!(size, 42);

			Ok(())
		}

		#[tokio::test]
		async fn test_router_client_uds_fanout_filters_by_wspace() -> Result<()> {
			let (socket_path, _watch, _rpc_rx, model_tx) =
				start_server("zc-router-test-uds-fanout.sock", Arc::new(PathStubResolver)).await?;

			let client_a = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-a")).await?;
			let client_b = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-b")).await?;

			let wspace_a = client_a.wspace_id().ok_or("missing wspace_a")?;
			let wspace_b = client_b.wspace_id().ok_or("missing wspace_b")?;
			assert_ne!(wspace_a, wspace_b);

			let mut model_rx_a = client_a.take_model_change_rx().ok_or("missing rx_a")?;
			let mut model_rx_b = client_b.take_model_change_rx().ok_or("missing rx_b")?;

			let model_event = ModelChangeEvent::new(
				EntityType::Run,
				EntityAction::Created,
				Some(Id::default()),
				RelIds::default(),
			);
			let event = RouterMsg {
				msg_id: MsgId::new(50),
				wspace_id: wspace_a,
				data: RouterMsgData::ModelChange(model_event),
			};
			model_tx.send(event).await?;

			let received_a = tokio::time::timeout(Duration::from_secs(1), model_rx_a.recv()).await??;
			assert_eq!(received_a.msg_id.as_u64(), 50);

			let timeout_b = tokio::time::timeout(Duration::from_millis(50), model_rx_b.recv()).await;
			assert!(
				timeout_b.is_err(),
				"client b should not have received event for wspace_a"
			);

			Ok(())
		}

		#[tokio::test]
		async fn test_router_client_uds_attach_err() -> Result<()> {
			let resolver = Arc::new(StubResolver {
				id: Id::default(),
				fail: true,
			});
			let (socket_path, mut watch, _rpc_rx, _model_tx) =
				start_server("zc-router-test-uds-attach-err.sock", resolver).await?;

			let res = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-err")).await;
			assert!(res.is_err());
			let err_msg = res.unwrap_err().to_string();
			assert!(
				err_msg.contains("stub resolver failure"),
				"expected resolver message, got: {err_msg}"
			);

			watch.wait_for_zero().await;
			assert_eq!(watch.count(), 0);

			Ok(())
		}

		#[tokio::test]
		async fn test_router_client_uds_send_stamps_wspace_id() -> Result<()> {
			let assigned_id = Id::try_from("00000000-0000-0000-0000-000000000088".to_string())?;
			let resolver = Arc::new(StubResolver {
				id: assigned_id,
				fail: false,
			});
			let (socket_path, _watch, _rpc_rx, _model_tx) =
				start_server("zc-router-test-uds-stamp.sock", resolver).await?;

			let client = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-stamp")).await?;
			let msg = RouterMsg::new(RouterMsgData::Exec(crate::exec::ExecCmd::RunPrompt("test".to_string())));
			assert_eq!(msg.wspace_id, Id::default());

			client.send(msg).await?;
			Ok(())
		}

		#[tokio::test]
		async fn test_router_client_uds_scoped_model_change_distribution() -> Result<()> {
			let (socket_path, _watch, _rpc_rx, model_tx) =
				start_server("zc-router-test-uds-scoped.sock", Arc::new(PathStubResolver)).await?;

			let client_a = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-a")).await?;
			let client_b = RouterClient::uds(&socket_path, ClientInfo::from_wspace_dir("/home/dev/zc-b")).await?;

			let wspace_a = client_a.wspace_id().ok_or("missing wspace_a")?;
			let mut rx_a = client_a.take_model_change_rx().ok_or("missing rx_a")?;
			let mut rx_b = client_b.take_model_change_rx().ok_or("missing rx_b")?;

			let event_data = ModelChangeEvent::new(
				EntityType::Run,
				EntityAction::Created,
				Some(Id::default()),
				RelIds {
					run_id: None,
					wspace_id: Some(wspace_a),
				},
			);
			let msg = RouterMsg {
				msg_id: MsgId::new(99),
				wspace_id: wspace_a,
				data: RouterMsgData::ModelChange(event_data),
			};
			model_tx.send(msg).await?;

			let recv_a = tokio::time::timeout(Duration::from_secs(1), rx_a.recv()).await??;
			assert_eq!(recv_a.wspace_id, wspace_a);

			let recv_b = tokio::time::timeout(Duration::from_millis(50), rx_b.recv()).await;
			assert!(recv_b.is_err());

			Ok(())
		}
	}
}

// endregion: --- Tests
