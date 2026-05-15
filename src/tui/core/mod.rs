#![allow(dead_code, unused_imports)]

// region:    --- Modules

mod app_event_handlers;
mod app_state;
mod event;
mod ping_timer;
mod term_reader;
mod tui_impl;
mod tui_loop;

pub mod types;

pub use app_state::AppState;
pub use event::{AppActionEvent, AppEvent};
pub use ping_timer::{PingTimerTx, start_ping_timer};
pub use tui_impl::{AppTx, start_tui};

// endregion: --- Modules
