// region:    --- Modules

mod derive_aliases;

use derive_aliases::*;

pub mod exec;
pub mod model;

// endregion: --- Modules

pub use model::Db;
