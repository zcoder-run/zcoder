use crate::Result;
use crossterm::event::{
	DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, KeyboardEnhancementFlags,
	PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::supports_keyboard_enhancement;
use ratatui::DefaultTerminal;
use std::io::stdout;

/// Terminal family, detected from the environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermKind {
	Kitty,
	Alacritty,
	Tmux,
	Other,
}

/// Sets up the terminal, and restores it on drop, including on early return and panic.
#[derive(Default)]
pub struct TermSetup {
	kbd_enhanced: bool,
}

impl TermSetup {
	/// Inits the terminal, enables mouse capture and bracketed paste, and enables keyboard
	/// enhancement when the terminal supports it, or when it is detected as Kitty or Alacritty,
	/// so Shift + Enter and Option + Enter are reported.
	/// Under tmux it stays off, since tmux does not forward the protocol.
	///
	/// The returned `TermSetup` must be kept alive while the TUI runs, since its drop is the teardown.
	pub fn init() -> Result<(DefaultTerminal, Self)> {
		let mut terminal = ratatui::init();
		let mut term_setup = Self::default();
		execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;

		let kind = detect_term_kind(|name| std::env::var(name).ok());
		let query_res = supports_keyboard_enhancement();
		// Without this, most macOS terminals report Option + Enter as a plain Enter.
		let kbd_enhanced = should_enable_kbd_enhancement(kind, matches!(query_res, Ok(true)));
		tracing::debug!(
			"->> terminal kind: {kind:?}, keyboard enhancement query: {query_res:?}, keyboard enhancement: {kbd_enhanced}"
		);
		if kbd_enhanced {
			// DISAMBIGUATE_ESCAPE_CODES alone keeps legacy bytes for Enter and Shift + key, so
			// Shift + Enter still arrives as a plain Enter. REPORT_ALL_KEYS_AS_ESCAPE_CODES encodes
			// Enter with its modifiers, and REPORT_ALTERNATE_KEYS keeps Shift + letter as uppercase.
			execute!(
				stdout(),
				PushKeyboardEnhancementFlags(
					KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
						| KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
						| KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
				)
			)?;
			term_setup.kbd_enhanced = true;
		}
		terminal.clear()?;

		Ok((terminal, term_setup))
	}
}

impl Drop for TermSetup {
	fn drop(&mut self) {
		if self.kbd_enhanced {
			let _ = execute!(stdout(), PopKeyboardEnhancementFlags);
		}
		let _ = execute!(stdout(), DisableMouseCapture, DisableBracketedPaste);
		ratatui::restore();
	}
}

// region:    --- Support

/// The query can fail even on terminals that support the protocol (timeout, or the reply
/// read by another reader), so Kitty and Alacritty detected from the environment are enabled too.
/// Terminals that do not know the push sequence ignore it.
/// Under tmux the flags are never pushed, since tmux does not forward the protocol to the
/// outer terminal, so Shift + Enter would not be reported anyway.
fn should_enable_kbd_enhancement(kind: TermKind, query_supported: bool) -> bool {
	kind != TermKind::Tmux && (query_supported || kind != TermKind::Other)
}

/// Detects the terminal family from environment variables.
///
/// `TMUX` is checked first, since tmux inherits the `ALACRITTY_*` variables of the window it
/// was started from, which would otherwise report Alacritty inside tmux.
/// The other variables are inherited through ssh, so the caller still confirms
/// keyboard enhancement support with a terminal query.
fn detect_term_kind(var: impl Fn(&str) -> Option<String>) -> TermKind {
	let term = var("TERM").unwrap_or_default();
	if var("TMUX").is_some() || term.starts_with("tmux") || term.starts_with("screen") {
		TermKind::Tmux
	} else if var("KITTY_WINDOW_ID").is_some() || term == "xterm-kitty" {
		TermKind::Kitty
	} else if var("ALACRITTY_WINDOW_ID").is_some() || var("ALACRITTY_SOCKET").is_some() || term == "alacritty" {
		TermKind::Alacritty
	} else {
		TermKind::Other
	}
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_core_term_setup_detect_term_kind_kitty() -> Result<()> {
		// -- Setup & Fixtures
		let by_id = env_from(&[("KITTY_WINDOW_ID", "1"), ("TERM", "xterm-256color")]);
		let by_term = env_from(&[("TERM", "xterm-kitty")]);

		// -- Exec & Check
		assert_eq!(detect_term_kind(by_id), TermKind::Kitty);
		assert_eq!(detect_term_kind(by_term), TermKind::Kitty);

		Ok(())
	}

	#[test]
	fn test_core_term_setup_detect_term_kind_alacritty() -> Result<()> {
		// -- Setup & Fixtures
		let by_id = env_from(&[("ALACRITTY_WINDOW_ID", "42"), ("TERM", "xterm-256color")]);
		let by_socket = env_from(&[("ALACRITTY_SOCKET", "/tmp/alacritty.sock")]);
		let by_term = env_from(&[("TERM", "alacritty")]);

		// -- Exec & Check
		assert_eq!(detect_term_kind(by_id), TermKind::Alacritty);
		assert_eq!(detect_term_kind(by_socket), TermKind::Alacritty);
		assert_eq!(detect_term_kind(by_term), TermKind::Alacritty);

		Ok(())
	}

	#[test]
	fn test_core_term_setup_detect_term_kind_tmux() -> Result<()> {
		// -- Setup & Fixtures
		let by_tmux = env_from(&[("TMUX", "/tmp/tmux-501/default,123,0"), ("ALACRITTY_WINDOW_ID", "42")]);
		let by_term = env_from(&[("TERM", "tmux-256color")]);

		// -- Exec & Check
		assert_eq!(detect_term_kind(by_tmux), TermKind::Tmux);
		assert_eq!(detect_term_kind(by_term), TermKind::Tmux);

		Ok(())
	}

	#[test]
	fn test_core_term_setup_detect_term_kind_other() -> Result<()> {
		// -- Setup & Fixtures
		let plain = env_from(&[("TERM", "xterm-256color"), ("TERM_PROGRAM", "Apple_Terminal")]);
		let empty = env_from(&[]);

		// -- Exec & Check
		assert_eq!(detect_term_kind(plain), TermKind::Other);
		assert_eq!(detect_term_kind(empty), TermKind::Other);

		Ok(())
	}

	#[test]
	fn test_core_term_setup_should_enable_kbd_enhancement() -> Result<()> {
		// -- Exec & Check
		assert!(should_enable_kbd_enhancement(TermKind::Other, true));
		assert!(should_enable_kbd_enhancement(TermKind::Alacritty, false));
		assert!(should_enable_kbd_enhancement(TermKind::Kitty, false));
		assert!(!should_enable_kbd_enhancement(TermKind::Other, false));
		assert!(!should_enable_kbd_enhancement(TermKind::Tmux, true));
		assert!(!should_enable_kbd_enhancement(TermKind::Tmux, false));

		Ok(())
	}

	// -- Test Support

	fn env_from(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
		let vars: Vec<(String, String)> = vars.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
		move |name| vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
	}
}

// endregion: --- Tests
