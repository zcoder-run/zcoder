// region:    --- Modules

pub mod config;
mod derive_aliases;
mod prompts;

use derive_aliases::*;

pub mod exec;
pub mod model;

// endregion: --- Modules

pub use config::{Config, ConfigManager};
pub use model::Db;
