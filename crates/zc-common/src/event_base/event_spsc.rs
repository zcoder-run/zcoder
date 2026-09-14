//! Single-producer single-consumer asynchronous channel endpoint wrappers.

use crate::event_base::event_base_error::{EventBaseError, EventBaseResult};
use crate::event_base::{DEFAULT_CAPACITY, support};
use crossfire::{AsyncRx, AsyncTx, TryRecvError, spsc};

// region:    --- Factories

/// Creates a bounded asynchronous SPSC channel with [`DEFAULT_CAPACITY`].
///
/// `name` is retained by both endpoints for diagnostics and disconnection
/// errors.
pub fn new_spsc_bounded_default<T>(name: &'static str) -> EventBaseResult<(SpscTx<T>, SpscRx<T>)>
where
	T: Send + 'static,
{
	new_spsc_bounded(name, DEFAULT_CAPACITY)
}

/// Creates a bounded asynchronous SPSC channel.
///
/// `capacity` is the number of queued messages and must be greater than zero.
/// A zero capacity returns [`EventBaseError::InvalidCapacity`].
pub fn new_spsc_bounded<T>(name: &'static str, capacity: usize) -> EventBaseResult<(SpscTx<T>, SpscRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(EventBaseError::InvalidCapacity { name, capacity });
	}
	let (tx, rx) = spsc::bounded_async::<T>(capacity);
	Ok((SpscTx { inner: tx, name }, SpscRx { inner: rx, name }))
}

// endregion: --- Factories

// region:    --- Spsc Implementations

/// SpSc SingleProducer sender. Not clonable, so it stays single-owner.
pub struct SpscTx<T: Send + 'static> {
	pub(super) inner: AsyncTx<spsc::Array<T>>,
	pub(super) name: &'static str,
}

impl<T: Send + 'static> std::fmt::Debug for SpscTx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SpscTx").field("name", &self.name).finish()
	}
}

/// SpSc SingleConsumer receiver. Not clonable, so it stays single-owner.
pub struct SpscRx<T: Send + 'static> {
	pub(super) inner: AsyncRx<spsc::Array<T>>,
	pub(super) name: &'static str,
}

impl<T: Send + 'static> std::fmt::Debug for SpscRx<T> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SpscRx").field("name", &self.name).finish()
	}
}

impl<T> SpscTx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Sends a message asynchronously, waiting until channel capacity is available.
	///
	/// `T: Unpin` is required by Crossfire's asynchronous send future. Callers that
	/// need to send a `!Unpin` value can use a movable pinned owner such as `Pin<Box<T>>`
	/// as the channel payload type.
	///
	/// If the receiver disconnects, returns [`EventBaseError::TxDisconnected`]
	/// and does not recover `message`. Cancelling the returned future before it
	/// completes leaves delivery unspecified.
	pub async fn send(&self, message: T) -> EventBaseResult<()>
	where
		T: Unpin,
	{
		support::handle_send_result(self.inner.send(message).await, self.name)
	}

	/// Attempts to send without blocking, returning the message when the channel is full.
	///
	/// Returns [`EventBaseError::TxDisconnected`] when the receiver has
	/// disconnected. The message is not recovered in that case.
	pub fn try_send(&self, message: T) -> EventBaseResult<Option<T>> {
		support::handle_try_send_result(self.inner.try_send(message), self.name)
	}

	/// Returns whether the receiver has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}
}

impl<T> SpscRx<T>
where
	T: Send + 'static,
{
	/// Returns the diagnostic name assigned when the channel was created.
	pub fn name(&self) -> &'static str {
		self.name
	}

	/// Mutable access keeps the receive future `Send` without requiring this single-consumer receiver to be `Sync`.
	///
	/// Returns [`EventBaseError::RxDisconnected`] after the sender has
	/// disconnected and no queued message remains.
	pub async fn recv(&mut self) -> EventBaseResult<T> {
		support::handle_recv_result(self.inner.recv().await, self.name)
	}

	/// Attempts to receive without blocking, returning `None` while the channel is empty.
	///
	/// Returns [`EventBaseError::RxDisconnected`] when the sender has
	/// disconnected.
	pub fn try_recv(&self) -> EventBaseResult<Option<T>> {
		match self.inner.try_recv() {
			// A message was immediately available.
			Ok(value) => Ok(Some(value)),

			// An empty, connected channel may receive a message later.
			Err(error @ TryRecvError::Empty) => support::handle_try_recv_error(error, self.name),

			// No message can arrive after the sender disconnects.
			Err(error @ TryRecvError::Disconnected) => support::handle_try_recv_error(error, self.name),
		}
	}

	/// Returns whether the sender has disconnected.
	pub fn is_disconnected(&self) -> bool {
		self.inner.is_disconnected()
	}
}

// endregion: --- Spsc Implementations

// region:    --- Tests

#[cfg(test)]
#[path = "event_spsc_tests.rs"]
mod tests;

// endregion: --- Tests

// region:    --- Spsc Blocking Send

impl<T> SpscTx<T>
where
	T: Send + 'static,
{
	/// Sends on the current thread, blocking only when the bounded channel is full.
	///
	/// The non-blocking attempt covers the common case without parking the thread. On
	/// backpressure, the asynchronous send is driven to completion on the current thread.
	///
	/// A single-owner SPSC sender cannot be cloned into a blocking handle the way the MPSC
	/// sender is, so the send future is polled here and this thread parks until the receiver
	/// frees capacity. `T: Unpin` is required because the underlying asynchronous send has
	/// that bound.
	///
	/// This blocks the current thread while the channel is full. Do not call it where
	/// blocking prevents the receiver from making progress. A disconnected receiver returns
	/// [`EventBaseError::TxDisconnected`] without recovering the message.
	pub fn send_sync(&self, message: T) -> EventBaseResult<()>
	where
		T: Unpin,
	{
		match self.try_send(message)? {
			// Capacity was available, so the message is already queued.
			None => Ok(()),

			// The channel is full, so the recovered message is sent by driving the
			// asynchronous send until the receiver frees capacity.
			Some(message) => block_on_result(self.send(message)),
		}
	}
}

// endregion: --- Spsc Blocking Send

// region:    --- Support

/// Drives `future` to completion on the current thread, parking until it is woken.
fn block_on_result<T>(future: impl core::future::Future<Output = EventBaseResult<T>>) -> EventBaseResult<T> {
	let waker = std::task::Waker::from(std::sync::Arc::new(ParkWaker(std::thread::current())));
	let mut context = std::task::Context::from_waker(&waker);
	let mut future = std::pin::pin!(future);

	// A wake from the channel unparks this thread, and a spurious wakeup simply polls again.
	loop {
		match core::future::Future::poll(future.as_mut(), &mut context) {
			std::task::Poll::Ready(result) => return result,
			std::task::Poll::Pending => std::thread::park(),
		}
	}
}

/// Unparks the thread that is blocked on a channel operation.
struct ParkWaker(std::thread::Thread);

impl std::task::Wake for ParkWaker {
	fn wake(self: std::sync::Arc<Self>) {
		self.0.unpark();
	}

	fn wake_by_ref(self: &std::sync::Arc<Self>) {
		self.0.unpark();
	}
}

// endregion: --- Support
