// region:    --- Modules

mod config_impl;
mod error;
mod manager;

pub use config_impl::{WksConfig, *};
pub use error::{Error, Result};
pub use manager::*;

// endregion: --- Modules
