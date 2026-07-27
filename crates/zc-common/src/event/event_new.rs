use crate::event::error::{Error, Result};
use crate::event::{MpmcRx, MpmcTx, MpscRx, MpscTx, OnceRx, OnceTx, SpscRx, SpscTx};
use crossfire::{mpmc, mpsc, oneshot, spsc};

// region:    --- Factory Functions

pub fn new_mpsc_bounded<T>(capacity: usize) -> Result<(MpscTx<T>, MpscRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(Error::InvalidCapacity(capacity));
	}
	let (tx, rx) = mpsc::bounded_async::<T>(capacity);
	Ok((MpscTx(tx), MpscRx(rx)))
}

pub fn new_mpmc_bounded<T>(capacity: usize) -> Result<(MpmcTx<T>, MpmcRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(Error::InvalidCapacity(capacity));
	}
	let (tx, rx) = mpmc::bounded_async::<T>(capacity);
	Ok((MpmcTx(tx), MpmcRx(rx)))
}

pub fn new_spsc_bounded<T>(capacity: usize) -> Result<(SpscTx<T>, SpscRx<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(Error::InvalidCapacity(capacity));
	}
	let (tx, rx) = spsc::bounded_async::<T>(capacity);
	Ok((SpscTx(tx), SpscRx(rx)))
}

pub fn new_once<T>() -> (OnceTx<T>, OnceRx<T>) {
	let (tx, rx) = oneshot::oneshot();
	(OnceTx(tx), OnceRx(rx))
}

// endregion: --- Factory Functions
