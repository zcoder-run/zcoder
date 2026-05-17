#![allow(unused)]

use crate::{Error, Result};
use crossfire::mpsc::Array;
use crossfire::{AsyncRx, MAsyncTx, RecvError, SendError, TryRecvError, TrySendError};

#[derive(Clone)]
pub struct Tx<T>
where
	T: Send + 'static,
{
	inner: MAsyncTx<Array<T>>,
}

pub struct Rx<T>
where
	T: Send + 'static,
{
	inner: AsyncRx<Array<T>>,
}

/// creates a new bounded mpsc channel with the given capacity and returns the sender and receiver.
pub fn new_mpsc_bounded<E>() -> (Tx<E>, Rx<E>)
where
	E: Send + 'static,
{
	let (ctx, crx) = crossfire::mpsc::bounded_async::<E>(10_000);
	let tx = Tx { inner: ctx };
	let rx = Rx { inner: crx };
	(tx, rx)
}

// region:    --- Implementation

impl<T> Tx<T>
where
	T: Send + 'static,
{
	pub async fn send(&self, msg: T) -> Result<()>
	where
		T: Unpin,
	{
		self.inner.send(msg).await.map_err(|e| Error::CrossfireSend(e.to_string()))
	}

	pub fn send_sync(&self, msg: T) -> Result<()> {
		let tx = self.inner.clone().into_blocking();
		tx.send(msg).map_err(|e| Error::CrossfireSend(e.to_string()))
	}

	pub fn try_send(&self, msg: T) -> Result<()> {
		let res = self.inner.try_send(msg);
		res.map_err(|e| Error::CrossfireSend(e.to_string()))
	}
}

impl<T> Rx<T>
where
	T: Send + 'static,
{
	pub async fn recv(&self) -> Result<T> {
		self.inner.recv().await.map_err(|e| Error::CrossfireRecv(e.to_string()))
	}

	pub fn try_recv(&self) -> Result<Option<T>> {
		match self.inner.try_recv() {
			Ok(res) => Ok(Some(res)),
			Err(err) => match err {
				TryRecvError::Empty => Ok(None),
				TryRecvError::Disconnected => Err(Error::CrossfireRecv("disconnected".to_string())),
			},
		}
	}
}

// endregion: --- Implementation
