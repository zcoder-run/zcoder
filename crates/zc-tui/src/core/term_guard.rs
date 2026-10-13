use crossterm::event::{DisableBracketedPaste, DisableMouseCapture};
use crossterm::execute;
use std::io::stdout;

/// Restores the terminal on drop, including on early return and panic.
pub struct TermGuard;

impl Drop for TermGuard {
	fn drop(&mut self) {
		let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
		ratatui::restore();
	}
}
