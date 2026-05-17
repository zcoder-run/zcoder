// region:    --- Modules

mod error;
mod event;
mod executor;

pub use error::{Error, Result};
pub use event::{ExecActionEvent, ExecStatusEvent};
pub use executor::{Executor, ExecutorConfig, ExecutorTx};

// endregion: --- Modules
