//! Core data types and event contracts for zcoder.
//!
//! This crate contains shared data structures, entity models, and event contracts.
//! It does not perform persistence or execution. The database and execution engine
//! are owned by `zc-base`.

// region:    --- Modules

mod derive_aliases;

use derive_aliases::*;

pub mod exec;
pub mod model;

// endregion: --- Modules
