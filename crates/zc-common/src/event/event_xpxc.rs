use crate::event::error::{Error, Result};
use crossfire::{AsyncRx, AsyncTx, MAsyncRx, MAsyncTx, TryRecvError, TrySendError, mpmc, mpsc, spsc};

// region:    --- Mpsc Implementations

/// MpSc MultiProducer sender. Clonable, allowing multiple senders.
#[derive(Clone)]
pub struct MpscTx<T: Send + 'static>(pub(super) MAsyncTx<mpsc::Array<T>>);

/// MpSc SingleConsumer receiver. Not clonable.
pub struct MpscRx<T: Send + 'static>(pub(super) AsyncRx<mpsc::Array<T>>);

impl<T> MpscTx<T>
where
	T: Send + 'static,
{
	pub async fn send(&self, message: T) -> Result<()>
	where
		T: Unpin,
	{
		self.0.send(message).await.map_err(|e| Error::Tx(e.to_string()))
	}

	/// Sends on the current thread, blocking only when the bounded channel is full.
	///
	/// The non-blocking attempt avoids converting sender modes while capacity is
	/// available. On backpressure, the recovered message is sent through a cloned
	/// blocking handle, preserving this sender for subsequent asynchronous use.
	pub fn send_sync(&self, message: T) -> Result<()> {
		match self.0.try_send(message) {
			Ok(()) => Ok(()),
			Err(TrySendError::Full(message)) => self
				.0
				.clone()
				.into_blocking()
				.send(message)
				.map_err(|e| Error::Tx(e.to_string())),
			Err(TrySendError::Disconnected(_)) => Err(Error::Tx("Channel disconnected".to_string())),
		}
	}
}

impl<T> MpscRx<T>
where
	T: Send + 'static,
{
	/// Mutable access keeps the receive future `Send` without requiring this single-consumer receiver to be `Sync`.
	pub async fn recv(&mut self) -> Result<T> {
		self.0.recv().await.map_err(|e| Error::Rx(e.to_string()))
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	pub fn try_recv(&self) -> Result<Option<T>> {
		match self.0.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(TryRecvError::Empty) => Ok(None),

			// No message can arrive after all senders disconnect.
			Err(TryRecvError::Disconnected) => Err(Error::Rx("Channel disconnected".to_string())),
		}
	}
}

// endregion: --- Mpsc Implementations

// region:    --- Mpmc Implementations

/// MpMc MultiProducer MultiConsumer sender. Clonable, allowing multiple senders.
#[derive(Clone)]
pub struct MpmcTx<T: Send + 'static>(pub(super) MAsyncTx<mpmc::Array<T>>);

/// MpMc MultiConsumer receiver. Not clonable.
pub struct MpmcRx<T: Send + 'static>(pub(super) MAsyncRx<mpmc::Array<T>>);

impl<T> Clone for MpmcRx<T>
where
	T: Send + 'static,
{
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}
}

impl<T> MpmcTx<T>
where
	T: Send + 'static,
{
	pub async fn send(&self, message: T) -> Result<()>
	where
		T: Unpin,
	{
		self.0.send(message).await.map_err(|e| Error::Tx(e.to_string()))
	}

	/// Sends on the current thread, blocking only when the bounded channel is full.
	///
	/// The non-blocking attempt avoids converting sender modes while capacity is
	/// available. On backpressure, the recovered message is sent through a cloned
	/// blocking handle, preserving this sender for subsequent asynchronous use.
	pub fn send_sync(&self, message: T) -> Result<()> {
		match self.0.try_send(message) {
			Ok(()) => Ok(()),
			Err(TrySendError::Full(message)) => self
				.0
				.clone()
				.into_blocking()
				.send(message)
				.map_err(|e| Error::Tx(e.to_string())),
			Err(TrySendError::Disconnected(_)) => Err(Error::Tx("Channel disconnected".to_string())),
		}
	}
}

impl<T> MpmcRx<T>
where
	T: Send + 'static,
{
	pub async fn recv(&self) -> Result<T> {
		self.0.recv().await.map_err(|e| Error::Rx(e.to_string()))
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	pub fn try_recv(&self) -> Result<Option<T>> {
		match self.0.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(TryRecvError::Empty) => Ok(None),

			// No message can arrive after all senders disconnect.
			Err(TryRecvError::Disconnected) => Err(Error::Rx("Channel disconnected".to_string())),
		}
	}
}

// endregion: --- Mpmc Implementations

// region:    --- Spsc Implementations

/// SpSc SingleProducer sender. Not clonable.
pub struct SpscTx<T: Send + 'static>(pub(super) AsyncTx<spsc::Array<T>>);

/// SpSc SingleConsumer receiver. Not clonable.
pub struct SpscRx<T: Send + 'static>(pub(super) AsyncRx<spsc::Array<T>>);

impl<T> SpscTx<T>
where
	T: Send + 'static,
{
	pub async fn send(&self, message: T) -> Result<()>
	where
		T: Unpin,
	{
		self.0.send(message).await.map_err(|e| Error::Tx(e.to_string()))
	}
}

impl<T> SpscRx<T>
where
	T: Send + 'static,
{
	/// Mutable access keeps the receive future `Send` without requiring this single-consumer receiver to be `Sync`.
	pub async fn recv(&mut self) -> Result<T> {
		self.0.recv().await.map_err(|e| Error::Rx(e.to_string()))
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	pub fn try_recv(&self) -> Result<Option<T>> {
		match self.0.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(TryRecvError::Empty) => Ok(None),

			// No message can arrive after the sender disconnects.
			Err(TryRecvError::Disconnected) => Err(Error::Rx("Channel disconnected".to_string())),
		}
	}
}

// endregion: --- Spsc Implementations
