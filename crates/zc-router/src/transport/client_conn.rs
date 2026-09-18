//! Client side socket connection carrying `RouterMsg` frames.
//!
//! The write half sits behind an async mutex, so concurrent callers cannot
//! interleave frame bytes. A background reader task decodes inbound frames and
//! hands them to a [`ClientConnSink`], which owns the correlation state and the
//! inbound event channels.

use super::wire::{WireReader, WireWriter};
use crate::error::Result;
use crate::model_rpc::ModelRpcReply;
use crate::msg::{RouterMsg, RouterMsgData};
use std::path::Path;
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::Mutex as AsyncMutex;
use tokio::task::JoinHandle;
use zc_common::MsgId;

// region:    --- Types

/// Receives decoded inbound frames from the connection reader task.
///
/// The sink owns the client correlation state, so the connection itself stays
/// free of pending-call and inbound-channel bookkeeping.
pub(crate) trait ClientConnSink: Send + Sync + 'static {
	/// Completes the pending call with this id, if one is registered.
	fn on_reply(&self, msg_id: MsgId, reply: ModelRpcReply);

	/// Forwards a base-originated model change message to the client channel.
	fn on_model_change(&self, msg: RouterMsg);

	/// Forwards a base-originated exec event message to the client channel.
	fn on_exec_event(&self, msg: RouterMsg);

	/// Marks the correlation state closed, so awaiting callers fail instead of
	/// waiting forever on a connection that ended.
	fn on_closed(&self);
}

/// One connected client socket, safe to send through concurrently.
pub(crate) struct ClientConn {
	writer: AsyncMutex<WireWriter<OwnedWriteHalf, RouterMsg>>,
	reader_task: JoinHandle<()>,
}

// endregion: --- Types

// region:    --- Constructors & Transport

impl ClientConn {
	/// Connects to the service socket and starts the background reader task.
	#[allow(dead_code)]
	pub(crate) async fn connect(
		label: impl Into<String>,
		socket_path: impl AsRef<Path>,
		sink: Arc<dyn ClientConnSink>,
	) -> Result<Self> {
		let stream = UnixStream::connect(socket_path.as_ref()).await?;
		Ok(Self::from_stream(label, stream, sink))
	}

	/// Wraps an already connected stream and starts the background reader task.
	pub(crate) fn from_stream(label: impl Into<String>, stream: UnixStream, sink: Arc<dyn ClientConnSink>) -> Self {
		let (reader, writer) = stream.into_split();
		let reader = WireReader::<_, RouterMsg>::new(reader);
		let writer = WireWriter::<_, RouterMsg>::new(writer);
		Self::from_halves(label, reader, writer, sink)
	}

	/// Wraps existing reader and writer halves and starts the background reader task.
	pub(crate) fn from_halves(
		label: impl Into<String>,
		reader: WireReader<OwnedReadHalf, RouterMsg>,
		writer: WireWriter<OwnedWriteHalf, RouterMsg>,
		sink: Arc<dyn ClientConnSink>,
	) -> Self {
		let reader_task = spawn_reader(label.into(), reader, sink);
		Self {
			writer: AsyncMutex::new(writer),
			reader_task,
		}
	}

	/// Writes one frame, holding the write half for the whole frame.
	///
	/// The mutex keeps concurrent callers from interleaving frame bytes.
	pub(crate) async fn send(&self, msg: RouterMsg) -> Result<()> {
		let mut writer = self.writer.lock().await;
		writer.write_frame(&msg).await
	}
}

impl Drop for ClientConn {
	fn drop(&mut self) {
		self.reader_task.abort();
	}
}

// endregion: --- Constructors & Transport

// region:    --- Support

fn spawn_reader(
	label: String,
	mut reader: WireReader<OwnedReadHalf, RouterMsg>,
	sink: Arc<dyn ClientConnSink>,
) -> JoinHandle<()> {
	tokio::spawn(async move {
		loop {
			match reader.read_frame().await {
				Ok(Some(msg)) => match msg {
					RouterMsg {
						msg_id,
						data: RouterMsgData::ModelRpcRes(reply),
						..
					} => sink.on_reply(msg_id, reply),
					frame @ RouterMsg {
						data: RouterMsgData::ModelChange(_),
						..
					} => sink.on_model_change(frame),
					frame @ RouterMsg {
						data: RouterMsgData::ExecEvent(_),
						..
					} => sink.on_exec_event(frame),
					_ => tracing::warn!("{label} - unexpected inbound frame on client connection"),
				},
				Ok(None) => break,
				Err(err) => {
					tracing::warn!("{label} - read error: {err}");
					break;
				}
			}
		}
		sink.on_closed();
	})
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::model_rpc::ModelRpcReq;
	use tokio::sync::mpsc;

	// region:    --- Support

	#[allow(clippy::large_enum_variant)]
	enum SinkEvent {
		Reply(MsgId, ModelRpcReply),
		ModelChange(MsgId),
		ExecEvent(MsgId),
		Closed,
	}

	struct TestSink {
		tx: mpsc::UnboundedSender<SinkEvent>,
	}

	impl ClientConnSink for TestSink {
		fn on_reply(&self, msg_id: MsgId, reply: ModelRpcReply) {
			let _ = self.tx.send(SinkEvent::Reply(msg_id, reply));
		}

		fn on_model_change(&self, msg: RouterMsg) {
			let _ = self.tx.send(SinkEvent::ModelChange(msg.msg_id));
		}

		fn on_exec_event(&self, msg: RouterMsg) {
			let _ = self.tx.send(SinkEvent::ExecEvent(msg.msg_id));
		}

		fn on_closed(&self) {
			let _ = self.tx.send(SinkEvent::Closed);
		}
	}

	// endregion: --- Support

	#[tokio::test]
	async fn test_client_conn_send_frame() -> Result<()> {
		// -- Setup & Fixtures
		let (conn_side, peer) = UnixStream::pair()?;
		let (tx_events, _rx_events) = mpsc::unbounded_channel();
		let conn = ClientConn::from_stream("test_send", conn_side, Arc::new(TestSink { tx: tx_events }));
		let mut peer_reader = WireReader::<_, RouterMsg>::new(peer);

		// -- Exec
		conn.send(RouterMsg::new(RouterMsgData::ModelRpcReq(ModelRpcReq::DbSize)))
			.await?;
		let actual = peer_reader.read_frame().await?.ok_or("missing frame")?;

		// -- Check
		assert!(matches!(actual.data, RouterMsgData::ModelRpcReq(ModelRpcReq::DbSize)));

		Ok(())
	}

	#[tokio::test]
	async fn test_client_conn_reader_demux_reply() -> Result<()> {
		// -- Setup & Fixtures
		let (conn_side, peer) = UnixStream::pair()?;
		let (tx_events, mut rx_events) = mpsc::unbounded_channel();
		let _conn = ClientConn::from_stream("test_demux_reply", conn_side, Arc::new(TestSink { tx: tx_events }));
		let (_peer_read, peer_write) = peer.into_split();
		let mut peer_writer = WireWriter::<_, RouterMsg>::new(peer_write);

		let expected_msg_id = MsgId::new(42);
		let frame = RouterMsg {
			msg_id: expected_msg_id,
			wks_id: Default::default(),
			data: RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(7))),
		};

		// -- Exec
		peer_writer.write_frame(&frame).await?;
		let event = rx_events.recv().await.ok_or("missing sink event")?;

		// -- Check
		match event {
			SinkEvent::Reply(msg_id, ModelRpcReply::DbSize(Ok(size))) => {
				assert_eq!(msg_id.as_u64(), expected_msg_id.as_u64());
				assert_eq!(size, 7);
			}
			_ => panic!("unexpected sink event"),
		}

		Ok(())
	}

	#[tokio::test]
	async fn test_client_conn_reader_ignores_unexpected_and_closes() -> Result<()> {
		// -- Setup & Fixtures
		let (conn_side, peer) = UnixStream::pair()?;
		let (tx_events, mut rx_events) = mpsc::unbounded_channel();
		let _conn = ClientConn::from_stream("test_unexpected", conn_side, Arc::new(TestSink { tx: tx_events }));
		let (_peer_read, peer_write) = peer.into_split();
		let mut peer_writer = WireWriter::<_, RouterMsg>::new(peer_write);

		// -- Exec: one unexpected frame, then close the peer write side
		peer_writer
			.write_frame(&RouterMsg::new(RouterMsgData::AttachErr("nope".to_string())))
			.await?;
		drop(peer_writer);

		let event = rx_events.recv().await.ok_or("missing sink event")?;

		// -- Check
		assert!(matches!(event, SinkEvent::Closed));

		Ok(())
	}
}

// endregion: --- Tests
