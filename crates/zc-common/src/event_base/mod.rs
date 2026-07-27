//! Application-owned channel facade over Crossfire.
//!
//! This layer exposes named, bounded MPSC, MPMC, SPSC, synchronous SPSC, and
//! one-shot channels without leaking Crossfire endpoint types. Operations
//! translate peer disconnections into [`EventBaseError`].

// region:    --- Modules

mod support;

mod event_base_error;
mod event_new;
mod event_once;
mod event_spsc;
mod event_xpxc;

pub use event_base_error::{EventBaseError, EventBaseResultResult};
pub use event_new::*;
pub use event_once::{OnceRx, OnceTx};
pub use event_spsc::*;
pub use event_xpxc::*;

// endregion: --- Modules
