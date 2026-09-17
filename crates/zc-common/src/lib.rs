// region:    --- Modules

mod error;
mod msg_id;

pub use error::{Error, Result};
pub use msg_id::MsgId;

pub mod cache;
pub mod consts;
pub mod dirs;
pub mod event_base;
pub mod jsons;
pub mod time;
pub mod yaml;

// endregion: --- Modules
