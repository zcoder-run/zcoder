// region:    --- Modules

mod event;
mod model_loop;
mod ping_timer;
pub(crate) mod sys_state;
mod term_reader;
mod tui_event_handlers;
mod tui_impl;
mod tui_loop;
mod tui_state;

pub mod types;

pub use tui_impl::start_tui;
pub use tui_state::TuiState;

// endregion: --- Modules
