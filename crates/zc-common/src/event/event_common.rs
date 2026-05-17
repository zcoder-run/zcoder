#![allow(unused)]

use crossfire::flavor::Flavor;
use crossfire::{AsyncRx, MAsyncTx, RecvError, SendError, TryRecvError, TrySendError};

#[derive(Clone)]
pub struct Tx<T>
where
	T: Flavor,
{
	inner: MAsyncTx<T>,
}

pub struct Rx<T>
where
	T: Flavor,
{
	inner: AsyncRx<T>,
}

// region:    --- Implementation

impl<F> Tx<F>
where
	F: Flavor,
{
	pub async fn send(&self, msg: F::Item) -> Result<(), SendError<F::Item>>
	where
		F::Item: Unpin,
	{
		self.inner.send(msg).await
	}

	pub fn send_sync(&self, msg: F::Item) -> Result<(), SendError<F::Item>> {
		let tx = self.inner.clone().into_blocking();
		tx.send(msg)
	}

	pub fn try_send(&self, msg: F::Item) -> Result<(), TrySendError<F::Item>> {
		self.inner.try_send(msg)
	}
}

impl<F> Rx<F>
where
	F: Flavor,
{
	pub async fn recv(&self) -> Result<F::Item, RecvError> {
		self.inner.recv().await
	}

	pub fn try_recv(&self) -> Result<F::Item, TryRecvError> {
		self.inner.try_recv()
	}
}

// endregion: --- Implementation
