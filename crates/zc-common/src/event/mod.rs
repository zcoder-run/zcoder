// region:    --- Modules

mod error;
mod event_new;
mod event_once;
mod event_xpxc;

// endregion: --- Modules

// region:    --- Re-exports

pub use error::{Error, Result};
pub use event_new::*;
pub use event_once::{OnceRx, OnceTx};
pub use event_xpxc::*;

// endregion: --- Re-exports
