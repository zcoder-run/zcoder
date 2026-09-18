// region:    --- Modules

use crate::client_filter::client_filter;
use crate::client_info::ClientInfo;
use crate::error::{Error, Result};
use crate::exec_event::ExecEventRx;
use crate::model_change::ModelChangeRx;
use crate::model_rpc::ModelRpcCmdTx;
use crate::msg::{RouterMsg, RouterMsgData, RouterMsgTx, new_router_msg_channel};
use crate::router::route;
use crate::transport::{WireReader, WireWriter, is_live, unlink_if_exists};
use crate::wks_resolver::WksResolver;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::watch;
use zc_core::exec::ExecCmdTx;
use zc_core::model::Id;

// endregion: --- Modules

// region:    --- Types

/// Bound socket service that accepts frontend connections and routes them to Core.
///
/// Each connection is served by its own task: it performs the inline attach
/// exchange, then forwards every other frame to [`route`]. Base-originated
/// events fan out to every attached connection that passes `client_filter`.
pub struct RouterServer {
	listener: UnixListener,
	exec_cmd_tx: ExecCmdTx,
	model_rpc_cmd_tx: ModelRpcCmdTx,
	wks_resolver: Arc<dyn WksResolver>,
	model_change_rx: ModelChangeRx,
	exec_event_rx: ExecEventRx,
	registry: ConnRegistry,
	conn_watch: ConnWatch,
}

/// Watches the attached connection count so the binary can run its idle shutdown timer.
///
/// The exit decision stays in the binary, because a library that calls
/// `std::process::exit` cannot be tested.
#[derive(Clone)]
pub struct ConnWatch {
	rx: watch::Receiver<usize>,
}

// endregion: --- Types

// region:    --- RouterServer

impl RouterServer {
	/// Binds the socket path, refusing when a live server already answers on it.
	///
	/// A path nobody answers on is stale and is unlinked before binding.
	pub async fn bind(
		socket_path: impl AsRef<Path>,
		exec_cmd_tx: ExecCmdTx,
		model_rpc_cmd_tx: ModelRpcCmdTx,
		wks_resolver: Arc<dyn WksResolver>,
		model_change_rx: ModelChangeRx,
		exec_event_rx: ExecEventRx,
	) -> Result<Self> {
		let socket_path = socket_path.as_ref();
		if is_live(socket_path).await {
			return Err(Error::custom(format!(
				"a live server already owns the socket path {}",
				socket_path.display()
			)));
		}
		unlink_if_exists(socket_path)?;
		let listener = UnixListener::bind(socket_path)?;
		let (registry, conn_watch) = ConnRegistry::new();

		Ok(Self {
			listener,
			exec_cmd_tx,
			model_rpc_cmd_tx,
			wks_resolver,
			model_change_rx,
			exec_event_rx,
			registry,
			conn_watch,
		})
	}

	/// Returns a handle that reports the attached connection count.
	///
	/// Call this before [`RouterServer::run`], which consumes the server.
	pub fn conn_watch(&self) -> ConnWatch {
		self.conn_watch.clone()
	}

	/// Runs the accept loop plus the two event fan-out loops.
	///
	/// Spawns one task per connection and returns only on an accept error.
	pub async fn run(self) -> Result<()> {
		let Self {
			listener,
			exec_cmd_tx,
			model_rpc_cmd_tx,
			wks_resolver,
			model_change_rx,
			exec_event_rx,
			registry,
			..
		} = self;

		// -- Fan base-originated events out to every matching connection.
		tokio::spawn(run_model_change_fanout(model_change_rx, registry.clone()));
		tokio::spawn(run_exec_event_fanout(exec_event_rx, registry.clone()));

		loop {
			let (stream, _addr) = listener.accept().await?;
			let exec_cmd_tx = exec_cmd_tx.clone();
			let model_rpc_cmd_tx = model_rpc_cmd_tx.clone();
			let wks_resolver = wks_resolver.clone();
			let registry = registry.clone();
			tokio::spawn(async move {
				if let Err(err) = handle_conn(stream, exec_cmd_tx, model_rpc_cmd_tx, wks_resolver, registry).await {
					tracing::error!("->> service connection error: {err}");
				}
			});
		}
	}
}

// endregion: --- RouterServer

// region:    --- ConnWatch

impl ConnWatch {
	/// Returns the current attached connection count.
	pub fn count(&self) -> usize {
		*self.rx.borrow()
	}

	/// Waits until no client is attached.
	pub async fn wait_for_zero(&mut self) {
		while *self.rx.borrow_and_update() != 0 {
			if self.rx.changed().await.is_err() {
				return;
			}
		}
	}

	/// Waits until at least one client is attached.
	pub async fn wait_for_nonzero(&mut self) {
		while *self.rx.borrow_and_update() == 0 {
			if self.rx.changed().await.is_err() {
				return;
			}
		}
	}
}

// endregion: --- ConnWatch

// region:    --- Support

/// Serves one connection: inline attach exchange, then route every other frame.
async fn handle_conn(
	stream: UnixStream,
	exec_cmd_tx: ExecCmdTx,
	model_rpc_cmd_tx: ModelRpcCmdTx,
	wks_resolver: Arc<dyn WksResolver>,
	registry: ConnRegistry,
) -> Result<()> {
	let (reader, writer) = stream.into_split();
	let mut reader = WireReader::<_, RouterMsg>::new(reader);
	let mut writer = WireWriter::<_, RouterMsg>::new(writer);
	let (res_tx, mut res_rx) = new_router_msg_channel();

	// -- Single writer task, so concurrent frames cannot interleave.
	let writer_task = tokio::spawn(async move {
		while let Ok(msg) = res_rx.recv().await {
			if let Err(err) = writer.write_frame(&msg).await {
				tracing::error!("->> connection write error: {err}");
				break;
			}
		}
	});

	// -- Attach exchange, inline on the fresh connection, before any other frame is routed.
	let Some(first) = reader.read_frame().await? else {
		drop(res_tx);
		let _ = writer_task.await;
		return Ok(());
	};

	let RouterMsgData::Attach(info) = first.data else {
		tracing::error!("->> first frame is not an attach, closing connection");
		drop(res_tx);
		let _ = writer_task.await;
		return Ok(());
	};

	let wks_id = match wks_resolver.resolve(&info).await {
		Ok(wks_id) => wks_id,
		Err(err) => {
			tracing::error!("->> attach failed for {}: {err}", info.wks_dir);
			let reply = RouterMsg {
				msg_id: first.msg_id,
				wks_id: Id::default(),
				data: RouterMsgData::AttachErr(err.reason()),
			};
			let _ = res_tx.send(reply).await;
			drop(res_tx);
			let _ = writer_task.await;
			return Ok(());
		}
	};

	// -- Register so base-originated events can reach this connection.
	let conn_guard = ConnGuard {
		registry: registry.clone(),
		conn_id: registry.register(wks_id, info, res_tx.clone()),
	};

	let ack = RouterMsg {
		msg_id: first.msg_id,
		wks_id,
		data: RouterMsgData::AttachOk(wks_id),
	};
	let _ = res_tx.send(ack).await;

	// -- Read loop, one routed message at a time.
	while let Some(msg) = reader.read_frame().await? {
		if let Err(err) = route(&exec_cmd_tx, &model_rpc_cmd_tx, &res_tx, msg).await {
			tracing::error!("->> route error: {err}");
		}
	}

	// Unregister before awaiting the writer, so dropping the registry's sender closes the channel.
	drop(conn_guard);
	drop(res_tx);
	let _ = writer_task.await;

	Ok(())
}

async fn run_model_change_fanout(mut model_change_rx: ModelChangeRx, registry: ConnRegistry) {
	while let Ok(msg) = model_change_rx.recv().await {
		registry.broadcast(msg).await;
	}
}

async fn run_exec_event_fanout(mut exec_event_rx: ExecEventRx, registry: ConnRegistry) {
	while let Ok(msg) = exec_event_rx.recv().await {
		registry.broadcast(msg).await;
	}
}

/// Connection identifier, unique per server run.
type ConnId = u64;

/// One attached connection: its workspace, its client info, and its response sender.
struct ConnEntry {
	wks_id: Id,
	#[allow(dead_code)]
	client_info: ClientInfo,
	res_tx: RouterMsgTx,
}

/// Shared registry of attached connections, plus the count watch the binary reads.
#[derive(Clone)]
struct ConnRegistry {
	inner: Arc<Mutex<ConnRegistryInner>>,
}

struct ConnRegistryInner {
	next_id: ConnId,
	conns: HashMap<ConnId, ConnEntry>,
	conn_count_tx: watch::Sender<usize>,
}

impl ConnRegistry {
	fn new() -> (Self, ConnWatch) {
		let (conn_count_tx, conn_count_rx) = watch::channel(0usize);
		let registry = Self {
			inner: Arc::new(Mutex::new(ConnRegistryInner {
				next_id: 1,
				conns: HashMap::new(),
				conn_count_tx,
			})),
		};
		(registry, ConnWatch { rx: conn_count_rx })
	}

	fn register(&self, wks_id: Id, client_info: ClientInfo, res_tx: RouterMsgTx) -> ConnId {
		let mut inner = self.inner.lock().unwrap_or_else(|err| err.into_inner());
		let conn_id = inner.next_id;
		inner.next_id += 1;
		tracing::debug!(
			"->> client attached conn_id={conn_id} wks_id={wks_id:?} wks_dir={}",
			client_info.wks_dir
		);
		inner.conns.insert(
			conn_id,
			ConnEntry {
				wks_id,
				client_info,
				res_tx,
			},
		);
		let count = inner.conns.len();
		let _ = inner.conn_count_tx.send(count);
		conn_id
	}

	fn remove(&self, conn_id: ConnId) {
		let mut inner = self.inner.lock().unwrap_or_else(|err| err.into_inner());
		if inner.conns.remove(&conn_id).is_some() {
			tracing::debug!("->> client detached conn_id={conn_id}");
			let count = inner.conns.len();
			let _ = inner.conn_count_tx.send(count);
		}
	}

	/// Sends one event to every attached connection that passes `client_filter`.
	///
	/// Delivery awaits each target channel in turn. With a handful of local
	/// clients and a writer task draining each channel, that keeps every event
	/// rather than dropping it when a channel is briefly full.
	async fn broadcast(&self, msg: RouterMsg) {
		let targets: Vec<RouterMsgTx> = {
			let inner = self.inner.lock().unwrap_or_else(|err| err.into_inner());
			inner
				.conns
				.values()
				.filter(|entry| client_filter(entry.wks_id, &msg))
				.map(|entry| entry.res_tx.clone())
				.collect()
		};

		for res_tx in targets {
			if let Err(err) = res_tx.send(msg.clone()).await {
				tracing::warn!("->> failed to deliver event to a connection: {err}");
			}
		}
	}
}

/// Removes the connection from the registry when the connection task ends.
struct ConnGuard {
	registry: ConnRegistry,
	conn_id: ConnId,
}

impl Drop for ConnGuard {
	fn drop(&mut self) {
		self.registry.remove(self.conn_id);
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::exec::ExecCmd;
	use futures_util::future::BoxFuture;
	use std::path::PathBuf;
	use zc_common::MsgId;

	/// Resolver stub returning a fixed id, or failing on demand.
	struct StubResolver {
		id: Id,
		fail: bool,
	}

	impl WksResolver for StubResolver {
		fn resolve<'a>(&'a self, _info: &'a ClientInfo) -> BoxFuture<'a, crate::error::Result<Id>> {
			Box::pin(async move {
				if self.fail {
					Err(Error::custom("stub resolver failure"))
				} else {
					Ok(self.id)
				}
			})
		}
	}

	#[tokio::test]
	async fn test_server_attach_ok() -> Result<()> {
		// -- Setup & Fixtures
		let wks_id = test_wks_id("1")?;
		let resolver: Arc<dyn WksResolver> = Arc::new(StubResolver {
			id: wks_id,
			fail: false,
		});
		let (socket_path, _watch) = start_test_server("zc-router-test-attach-ok.sock", resolver).await?;
		let stream = UnixStream::connect(&socket_path).await?;
		let (read_half, write_half) = stream.into_split();
		let mut reader = WireReader::<_, RouterMsg>::new(read_half);
		let mut writer = WireWriter::<_, RouterMsg>::new(write_half);

		// -- Exec
		writer.write_frame(&attach_msg(1, "/home/dev/zc-wks")).await?;
		let reply = reader.read_frame().await?.ok_or("missing attach reply")?;

		// -- Check
		assert_eq!(reply.msg_id.as_u64(), 1);
		match reply.data {
			RouterMsgData::AttachOk(id) => assert_eq!(id, wks_id),
			_ => panic!("unexpected attach reply"),
		}

		Ok(())
	}

	#[tokio::test]
	async fn test_server_attach_err_closes_connection() -> Result<()> {
		// -- Setup & Fixtures
		let resolver: Arc<dyn WksResolver> = Arc::new(StubResolver {
			id: Id::default(),
			fail: true,
		});
		let (socket_path, watch) = start_test_server("zc-router-test-attach-err.sock", resolver).await?;
		let stream = UnixStream::connect(&socket_path).await?;
		let (read_half, write_half) = stream.into_split();
		let mut reader = WireReader::<_, RouterMsg>::new(read_half);
		let mut writer = WireWriter::<_, RouterMsg>::new(write_half);

		// -- Exec
		writer.write_frame(&attach_msg(2, "/home/dev/zc-wks")).await?;
		let reply = reader.read_frame().await?.ok_or("missing attach reply")?;

		// -- Check
		assert_eq!(reply.msg_id.as_u64(), 2);
		match reply.data {
			RouterMsgData::AttachErr(message) => assert_eq!(message, "stub resolver failure"),
			_ => panic!("unexpected attach reply"),
		}
		assert_eq!(watch.count(), 0);

		Ok(())
	}

	#[tokio::test]
	async fn test_server_registry_count_tracks_connections() -> Result<()> {
		// -- Setup & Fixtures
		let (registry, watch) = ConnRegistry::new();
		assert_eq!(watch.count(), 0);

		// -- Exec: register
		let (res_tx, _res_rx) = new_router_msg_channel();
		let conn_id = registry.register(Id::default(), ClientInfo::from_wks_dir("/home/dev/zc-wks"), res_tx);

		// -- Check
		assert_eq!(watch.count(), 1);

		// -- Exec: remove
		registry.remove(conn_id);

		// -- Check
		assert_eq!(watch.count(), 0);

		Ok(())
	}

	#[tokio::test]
	async fn test_server_registry_broadcast_filters_by_wks_id() -> Result<()> {
		// -- Setup & Fixtures
		let (registry, _watch) = ConnRegistry::new();
		let wks_a = test_wks_id("1")?;
		let wks_b = test_wks_id("2")?;
		let (tx_b, mut rx_b) = new_router_msg_channel();
		registry.register(wks_b, ClientInfo::from_wks_dir("/home/dev/zc-b"), tx_b);

		// -- Exec: one event for another workspace, then one for this workspace
		registry.broadcast(make_exec_msg(1, wks_a, "other")).await;
		registry.broadcast(make_exec_msg(2, wks_b, "mine")).await;

		// -- Check: only the matching event arrives
		let received = rx_b.recv().await?;
		assert_eq!(received.msg_id.as_u64(), 2);

		Ok(())
	}

	// region:    --- Test Support

	async fn start_test_server(socket_name: &str, resolver: Arc<dyn WksResolver>) -> Result<(PathBuf, ConnWatch)> {
		let socket_path = std::env::temp_dir().join(socket_name);
		unlink_if_exists(&socket_path)?;

		let (exec_cmd_tx, exec_cmd_rx) = zc_common::event_base::new_mpsc_bounded("test_exec_cmd", 8)?;
		let (model_rpc_cmd_tx, model_rpc_cmd_rx) = crate::model_rpc::new_model_rpc_cmd_channel();
		let (model_change_tx, model_change_rx) = crate::model_change::new_model_change_channel();
		let (exec_event_tx, exec_event_rx) = crate::exec_event::new_exec_event_channel();

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

		// Hold the core-side channel ends open for the lifetime of the test.
		tokio::spawn(async move {
			let (_exec_cmd_rx, _model_rpc_cmd_rx, _model_change_tx, _exec_event_tx) =
				(exec_cmd_rx, model_rpc_cmd_rx, model_change_tx, exec_event_tx);
			std::future::pending::<()>().await;
		});

		Ok((socket_path, watch))
	}

	fn attach_msg(msg_id: u64, wks_dir: &str) -> RouterMsg {
		RouterMsg {
			msg_id: MsgId::new(msg_id),
			wks_id: Id::default(),
			data: RouterMsgData::Attach(ClientInfo::from_wks_dir(wks_dir)),
		}
	}

	fn make_exec_msg(msg_id: u64, wks_id: Id, prompt: &str) -> RouterMsg {
		RouterMsg {
			msg_id: MsgId::new(msg_id),
			wks_id,
			data: RouterMsgData::Exec(ExecCmd::RunPrompt(prompt.to_string())),
		}
	}

	fn test_wks_id(tail: &str) -> Result<Id> {
		let uuid = format!("00000000-0000-0000-0000-{tail:0>12}");
		Ok(Id::try_from(uuid)?)
	}

	// endregion: --- Test Support
}

// endregion: --- Tests
