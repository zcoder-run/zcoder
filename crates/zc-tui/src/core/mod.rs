// region:    --- Modules

mod app_event_handlers;
mod model_loop;
mod ping_timer;
mod term_reader;
mod tui_impl;
mod tui_loop;
mod tui_state;

pub mod types;

pub use tui_impl::start_tui;
pub use tui_state::TuiState;

// endregion: --- Modules
