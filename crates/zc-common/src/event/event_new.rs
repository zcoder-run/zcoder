use crate::event::error::{Error, Result};
use crate::event::{Mp, Sc};
use crossfire::mpsc;

// region:    --- Factory Functions

// pub fn new_mpsc_unbounded<T>() -> Result<(Mp<T>, Sc<T>)>
// where
// 	T: Send + 'static,
// {
// 	let (tx, rx) = mpsc::unbounded_async::<T>();
// 	Ok((Mp(tx), Sc(rx)))
// }

pub fn new_mpsc_bounded<T>(capacity: usize) -> Result<(Mp<T>, Sc<T>)>
where
	T: Send + 'static,
{
	if capacity == 0 {
		return Err(Error::InvalidCapacity(capacity));
	}
	let (tx, rx) = mpsc::bounded_async::<T>(capacity);
	Ok((Mp(tx), Sc(rx)))
}

// pub fn new_mpmc_bounded<T>(capacity: usize) -> Result<(Mp<T>, Mc<T>)>
// where
// 	T: Send + 'static,
// {
// 	if capacity == 0 {
// 		return Err(Error::InvalidCapacity(capacity));
// 	}
// 	let (tx, rx) = mpmc::bounded_async::<T>(capacity);
// 	Ok((Mp(tx), Mc(rx)))
// }

// pub fn new_mpmc_unbounded<T>() -> Result<(Mp<T>, Mc<T>)> {
// 	let (tx, rx) = mpmc::unbounded_async();
// 	Ok((Mp(tx), Mc(rx)))
// }

// pub fn new_spsc_unbounded<T>() -> Result<(Sp<T>, Sc<T>)> {
// 	let (tx, rx) = spsc::unbounded_async();
// 	Ok((Sp(tx), Sc(rx)))
// }

// pub fn new_spsc_bounded<T>(capacity: usize) -> Result<(Sp<T>, Sc<T>)> {
// 	if capacity == 0 {
// 		return Err(Error::InvalidCapacity(capacity));
// 	}
// 	let (tx, rx) = spsc::bounded_async(capacity);
// 	Ok((Sp(tx), Sc(rx)))
// }

// pub fn new_oneshot<T>() -> Result<(OnceSp<T>, OnceSc<T>)> {
// 	let (tx, rx) = oneshot::oneshot();
// 	Ok((OnceSp(tx), OnceSc(rx)))
// }

// endregion: --- Factory Functions
