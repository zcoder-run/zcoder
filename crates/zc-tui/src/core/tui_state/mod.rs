// region:    --- Modules

mod prompter_layout;
mod prompter_state;
mod state_processor;
mod tui_state_base;

pub use prompter_state::{PROMPTER_MAX_ROWS, PrompterState};
#[allow(unused_imports)]
pub use state_processor::{PromptSubmit, StateProcessor};
pub use tui_state_base::TuiState;

// endregion: --- Modules
