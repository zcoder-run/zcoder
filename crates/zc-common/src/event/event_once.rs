use crate::event::error::{Error, Result};
use crossfire::oneshot;

// region:    --- Types

/// Single-use producer, consumed on send.
pub struct OnceSp<T>(pub(crate) oneshot::TxOneshot<T>);

/// Single-use consumer, consumed on recv.
pub struct OnceSc<T>(pub(crate) oneshot::RxOneshot<T>);

// endregion: --- Types

// region:    --- Implementation OnceSp<T>

impl<T> OnceSp<T> {
	pub fn send(self, message: T) -> Result<()> {
		self.0.send(message);
		// Note: no error on this send, but still
		Ok(())
	}
}

// endregion: --- Implementation OnceSp<T>

// region:    --- Implementation OnceSc<T>

impl<T> OnceSc<T> {
	pub async fn recv(self) -> Result<T> {
		self.0.recv_async().await.map_err(|error| Error::Rx(error.to_string()))
	}

	// TODO: do the recv_sync(self), and try_recv
}

// endregion: --- Implementation OnceSc<T>
