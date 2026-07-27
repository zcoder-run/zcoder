use crate::event::error::{Error, Result};
use crossfire::oneshot;

// region:    --- Implementation OnceTx<T>

/// Single-use producer, consumed on send.
pub struct OnceTx<T>(pub(super) oneshot::TxOneshot<T>);

impl<T> OnceTx<T> {
	pub fn send(self, message: T) {
		self.0.send(message);
	}
}

// endregion: --- Implementation OnceTx<T>

// region:    --- Implementation OnceRx<T>

/// Single-use consumer, consumed on recv.
pub struct OnceRx<T>(pub(super) oneshot::RxOneshot<T>);

impl<T> OnceRx<T> {
	pub async fn recv(self) -> Result<T> {
		self.0.recv_async().await.map_err(|error| Error::Rx(error.to_string()))
	}
}

// endregion: --- Implementation OnceRx<T>
