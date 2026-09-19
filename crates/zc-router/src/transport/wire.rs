//! Length-delimited framing over any async byte stream.
//!
//! Frame layout: `[u32 payload length, little-endian][postcard payload bytes]`.

use crate::error::{Error, Result};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

/// Upper bound on a single payload, guards against a bogus length prefix.
pub const MAX_FRAME_LEN: u32 = 8 * 1024 * 1024;

// region:    --- Types

pub(crate) struct WireReader<R, T> {
	framed: FramedRead<R, LengthDelimitedCodec>,
	marker: PhantomData<fn() -> T>,
}

pub(crate) struct WireWriter<W, T> {
	framed: FramedWrite<W, LengthDelimitedCodec>,
	marker: PhantomData<fn() -> T>,
}

// endregion: --- Types

// region:    --- Support

/// Builds the codec: a little-endian `u32` payload length followed by the payload.
fn new_codec() -> LengthDelimitedCodec {
	LengthDelimitedCodec::builder()
		.length_field_type::<u32>()
		.length_field_offset(0)
		.little_endian()
		.length_adjustment(0)
		.num_skip(4)
		.max_frame_length(MAX_FRAME_LEN as usize)
		.new_codec()
}

// endregion: --- Support

// region:    --- WireReader

impl<R, T> WireReader<R, T>
where
	R: AsyncRead + Unpin,
{
	pub(crate) fn new(reader: R) -> Self {
		Self {
			framed: FramedRead::new(reader, new_codec()),
			marker: PhantomData,
		}
	}
}

impl<R, T> WireReader<R, T>
where
	R: AsyncRead + Unpin,
	T: DeserializeOwned,
{
	/// Reads one length-delimited frame and decodes it with postcard.
	///
	/// Returns `Ok(None)` on a clean end of stream, that is, when the peer
	/// closed the connection on a frame boundary.
	pub(crate) async fn read_frame(&mut self) -> Result<Option<T>> {
		match self.framed.next().await {
			Some(Ok(payload)) => Ok(Some(postcard::from_bytes(&payload)?)),
			Some(Err(err)) => Err(err.into()),
			None => Ok(None),
		}
	}
}

// endregion: --- WireReader

// region:    --- WireWriter

impl<W, T> WireWriter<W, T>
where
	W: AsyncWrite + Unpin,
{
	pub(crate) fn new(writer: W) -> Self {
		Self {
			framed: FramedWrite::new(writer, new_codec()),
			marker: PhantomData,
		}
	}
}

impl<W, T> WireWriter<W, T>
where
	W: AsyncWrite + Unpin,
	T: Serialize,
{
	/// Serializes `value` with postcard and writes it as one length-delimited frame.
	pub(crate) async fn write_frame(&mut self, value: &T) -> Result<()> {
		let payload = postcard::to_stdvec(value)?;
		if payload.len() > MAX_FRAME_LEN as usize {
			return Err(Error::FrameTooLarge {
				len: payload.len(),
				max: MAX_FRAME_LEN,
			});
		}

		self.framed.send(payload.into()).await?;
		self.framed.flush().await?;

		Ok(())
	}
}

// endregion: --- WireWriter

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::model_rpc::ModelRpcReply;
	use crate::msg::{RouterMsg, RouterMsgData};
	use tokio::io::{AsyncReadExt, AsyncWriteExt};

	#[tokio::test]
	async fn test_transport_wire_read_clean_eof() -> Result<()> {
		// -- Setup & Fixtures
		let mut reader = WireReader::<_, u8>::new(tokio::io::empty());

		// -- Exec
		let actual = reader.read_frame().await?;

		// -- Check
		assert!(actual.is_none());

		Ok(())
	}

	#[tokio::test]
	async fn test_transport_wire_read_partial_length_prefix() -> Result<()> {
		// -- Setup & Fixtures
		let (mut writer, reader_io) = tokio::io::duplex(64);
		writer.write_all(&[0x01, 0x02]).await?;
		drop(writer);
		let mut reader = WireReader::<_, u8>::new(reader_io);

		// -- Exec & Check
		assert!(reader.read_frame().await.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_transport_wire_read_truncated_payload() -> Result<()> {
		// -- Setup & Fixtures
		let (mut writer, reader_io) = tokio::io::duplex(64);
		writer.write_all(&3u32.to_le_bytes()).await?;
		writer.write_all(&[0x01, 0x02]).await?;
		drop(writer);
		let mut reader = WireReader::<_, u8>::new(reader_io);

		// -- Exec & Check
		assert!(reader.read_frame().await.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_transport_wire_reject_oversized_frame() -> Result<()> {
		// -- Setup & Fixtures
		let (mut writer, reader_io) = tokio::io::duplex(64);
		writer.write_all(&(MAX_FRAME_LEN + 1).to_le_bytes()).await?;
		drop(writer);
		let mut reader = WireReader::<_, u8>::new(reader_io);

		// -- Exec & Check
		assert!(reader.read_frame().await.is_err());

		Ok(())
	}

	#[tokio::test]
	async fn test_transport_wire_multiple_buffered_frames() -> Result<()> {
		// -- Setup & Fixtures
		let (writer_io, reader_io) = tokio::io::duplex(128);
		let mut writer = WireWriter::<_, u32>::new(writer_io);
		writer.write_frame(&11).await?;
		writer.write_frame(&22).await?;
		drop(writer);
		let mut reader = WireReader::<_, u32>::new(reader_io);

		// -- Exec
		let first = reader.read_frame().await?.ok_or("missing first frame")?;
		let second = reader.read_frame().await?.ok_or("missing second frame")?;
		let end = reader.read_frame().await?;

		// -- Check
		assert_eq!(first, 11);
		assert_eq!(second, 22);
		assert!(end.is_none());

		Ok(())
	}

	#[tokio::test]
	async fn test_transport_wire_little_endian_prefix() -> Result<()> {
		// -- Setup & Fixtures
		let value = 42u8;
		let payload = postcard::to_stdvec(&value)?;
		let (writer_io, reader_io) = tokio::io::duplex(64);
		let mut writer = WireWriter::<_, u8>::new(writer_io);
		let mut reader = reader_io;

		// -- Exec
		writer.write_frame(&value).await?;
		drop(writer);
		let mut prefix = [0u8; 4];
		reader.read_exact(&mut prefix).await?;
		let mut rest = Vec::new();
		reader.read_to_end(&mut rest).await?;

		// -- Check
		assert_eq!(prefix, [1, 0, 0, 0]);
		assert_eq!(&rest[rest.len() - payload.len()..], payload.as_slice());

		Ok(())
	}

	#[tokio::test]
	async fn test_transport_wire_router_msg_round_trip() -> Result<()> {
		// -- Setup & Fixtures
		let msg = RouterMsg::new(RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(42))));
		let (writer_io, reader_io) = tokio::io::duplex(256);
		let mut writer = WireWriter::<_, RouterMsg>::new(writer_io);
		let mut reader = WireReader::<_, RouterMsg>::new(reader_io);

		// -- Exec
		writer.write_frame(&msg).await?;
		let actual = reader.read_frame().await?.ok_or("missing router msg frame")?;

		// -- Check
		assert_eq!(actual.msg_id, msg.msg_id);
		assert_eq!(actual.wspace_id, msg.wspace_id);
		assert!(matches!(
			actual.data,
			RouterMsgData::ModelRpcRes(ModelRpcReply::DbSize(Ok(42)))
		));

		Ok(())
	}
}

// endregion: --- Tests
