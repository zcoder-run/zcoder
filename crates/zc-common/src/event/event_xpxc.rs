use crate::event::error::{Error, Result};
use crossfire::mpsc::Array;
use crossfire::{AsyncRx, AsyncTx, MAsyncRx, MAsyncTx, TryRecvError, TrySendError};

// region:    --- Types

/// MultiProducer sender. Clonable, allowing multiple senders.
#[derive(Clone)]
pub struct Mp<T: Send + 'static>(pub(super) MAsyncTx<Array<T>>);

/// SingleProducer sender. Not clonable.
pub struct Sp<T: Send + 'static>(pub(super) AsyncTx<Array<T>>);

/// SingleConsumer receiver. Not clonable.
pub struct Sc<T: Send + 'static>(pub(super) AsyncRx<Array<T>>);

/// MultiConsumer receiver. Clonable, allowing competing consumers.
#[derive(Clone)]
pub struct Mc<T: Send + 'static>(pub MAsyncRx<Array<T>>);

// endregion: --- Types

// region:    --- Implementation Mp<T>

impl<T> Mp<T>
where
	T: Send + 'static,
{
	pub async fn send(&self, message: T) -> Result<()>
	where
		T: Unpin,
	{
		self.0.send(message).await.map_err(|e| Error::Tx(e.to_string()))
	}

	pub fn send_sync(&self, message: T) -> Result<()> {
		match self.0.try_send(message) {
			Ok(()) => Ok(()),
			Err(TrySendError::Full(message)) => {
				let blocking_sender = self.0.clone().into_blocking();
				blocking_sender.send(message).map_err(|e| Error::Tx(e.to_string()))
			}
			Err(TrySendError::Disconnected(_)) => Err(Error::Tx("Channel disconnected".to_string())),
		}
	}
}

// endregion: --- Implementation Mp<T>

// region:    --- Implementation Sp<T>

impl<T> Sp<T>
where
	T: Send + 'static,
{
	pub async fn send(&self, message: T) -> Result<()>
	where
		T: Unpin,
	{
		self.0.send(message).await.map_err(|e| Error::Tx(e.to_string()))
	}

	// TODO: needs to implement send_sync
}

// endregion: --- Implementation Sp<T>

// region:    --- Implementation Sc<T>

impl<T> Sc<T>
where
	T: Send + 'static,
{
	pub async fn recv(&mut self) -> Result<T> {
		self.0.recv().await.map_err(|e| Error::Rx(e.to_string()))
	}

	pub fn try_recv(&self) -> Result<Option<T>> {
		match self.0.try_recv() {
			Ok(value) => Ok(Some(value)),
			Err(TryRecvError::Empty) => Ok(None),
			Err(TryRecvError::Disconnected) => Err(Error::Rx("Channel disconnected".to_string())),
		}
	}
}

// endregion: --- Implementation Sc<T>

// region:    --- Implementation Mc<T>

impl<T> Mc<T>
where
	T: Send + 'static,
{
	pub async fn recv(&mut self) -> Result<T> {
		self.0.recv().await.map_err(|e| Error::Rx(e.to_string()))
	}

	pub fn try_recv(&self) -> Result<Option<T>> {
		match self.0.try_recv() {
			Ok(value) => Ok(Some(value)),
			Err(TryRecvError::Empty) => Ok(None),
			Err(TryRecvError::Disconnected) => Err(Error::Rx("Channel disconnected".to_string())),
		}
	}
}

// endregion: --- Implementation Mc<T>
