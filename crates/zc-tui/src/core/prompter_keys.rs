use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Cursor motion used by `PrompterAction::Move`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrompterMotion {
	Left,
	Right,
	WordLeft,
	WordRight,
	Up,
	Down,
	RowStart,
	RowEnd,
	TextStart,
	TextEnd,
}

/// Prompter key action, applied to `PrompterState` or handled by the event handler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrompterAction {
	Insert(char),
	Newline,
	Backspace,
	Delete,
	Move { motion: PrompterMotion, select: bool },
	SelectAll,
	ClearSelection,
	Copy,
	Paste,
}

/// Returns true when the key sends the prompt (Enter, or Ctrl + Enter when reported).
///
/// - Shift + Enter and Alt + Enter (Option + Enter on macOS) insert a newline instead. They are
///   reported only with keyboard enhancement (such as Kitty and Alacritty), or Alt with "Option as Meta".
/// - A `\` before the cursor turns the send into a newline, see `PrompterState::continue_line`.
pub fn is_send_key(key: &KeyEvent) -> bool {
	key.code == KeyCode::Enter && !key.modifiers.intersects(KeyModifiers::SHIFT | KeyModifiers::ALT)
}

/// Maps a key event to a prompter action, or `None` when the key is not a prompter key.
pub fn map_prompter_key(key: &KeyEvent) -> Option<PrompterAction> {
	let mods = key.modifiers;
	let ctrl = mods.contains(KeyModifiers::CONTROL);
	let alt = mods.contains(KeyModifiers::ALT);
	let select = mods.contains(KeyModifiers::SHIFT);
	let word = ctrl || alt;

	let action = match key.code {
		KeyCode::Enter if select || alt => PrompterAction::Newline,
		KeyCode::Char('j') if ctrl => PrompterAction::Newline,
		KeyCode::Backspace => PrompterAction::Backspace,
		KeyCode::Delete => PrompterAction::Delete,
		KeyCode::Left if word => move_action(PrompterMotion::WordLeft, select),
		KeyCode::Right if word => move_action(PrompterMotion::WordRight, select),
		KeyCode::Left => move_action(PrompterMotion::Left, select),
		KeyCode::Right => move_action(PrompterMotion::Right, select),
		KeyCode::Up if word => move_action(PrompterMotion::TextStart, select),
		KeyCode::Down if word => move_action(PrompterMotion::TextEnd, select),
		KeyCode::Up => move_action(PrompterMotion::Up, select),
		KeyCode::Down => move_action(PrompterMotion::Down, select),
		KeyCode::Home if !ctrl => move_action(PrompterMotion::RowStart, select),
		KeyCode::End if !ctrl => move_action(PrompterMotion::RowEnd, select),
		KeyCode::Esc => PrompterAction::ClearSelection,
		KeyCode::Char('a') if ctrl => PrompterAction::SelectAll,
		KeyCode::Char('v') if ctrl => PrompterAction::Paste,
		KeyCode::Char('c') if alt => PrompterAction::Copy,
		KeyCode::Char('b') if alt => move_action(PrompterMotion::WordLeft, false),
		KeyCode::Char('f') if alt => move_action(PrompterMotion::WordRight, false),
		KeyCode::Char(c) if !ctrl && !alt => PrompterAction::Insert(c),
		_ => return None,
	};

	Some(action)
}

// region:    --- Support

fn move_action(motion: PrompterMotion, select: bool) -> PrompterAction {
	PrompterAction::Move { motion, select }
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_core_prompter_keys_map_movement() -> Result<()> {
		// -- Setup & Fixtures
		let cases = [
			(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE), move_action(PrompterMotion::Left, false)),
			(KeyEvent::new(KeyCode::Left, KeyModifiers::SHIFT), move_action(PrompterMotion::Left, true)),
			(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL), move_action(PrompterMotion::WordRight, false)),
			(
				KeyEvent::new(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::SHIFT),
				move_action(PrompterMotion::WordLeft, true),
			),
			(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE), move_action(PrompterMotion::Up, false)),
			(KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT), move_action(PrompterMotion::Down, true)),
			(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL), move_action(PrompterMotion::TextStart, false)),
			(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT), move_action(PrompterMotion::TextEnd, false)),
			(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE), move_action(PrompterMotion::RowStart, false)),
			(KeyEvent::new(KeyCode::End, KeyModifiers::SHIFT), move_action(PrompterMotion::RowEnd, true)),
		];

		// -- Exec & Check
		for (key, expected) in cases {
			assert_eq!(map_prompter_key(&key), Some(expected));
		}

		Ok(())
	}

	#[test]
	fn test_core_prompter_keys_map_shortcuts() -> Result<()> {
		// -- Setup & Fixtures
		let cases = [
			(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL), PrompterAction::SelectAll),
			(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL), PrompterAction::Paste),
			(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT), PrompterAction::Copy),
			(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::ALT), move_action(PrompterMotion::WordLeft, false)),
			(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::ALT), move_action(PrompterMotion::WordRight, false)),
			(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), PrompterAction::ClearSelection),
		];

		// -- Exec & Check
		for (key, expected) in cases {
			assert_eq!(map_prompter_key(&key), Some(expected));
		}
		assert_eq!(map_prompter_key(&KeyEvent::new(KeyCode::Home, KeyModifiers::CONTROL)), None);

		Ok(())
	}

	#[test]
	fn test_core_prompter_keys_map_insert_and_edit() -> Result<()> {
		// -- Setup & Fixtures
		let cases = [
			(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE), PrompterAction::Insert('x')),
			(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT), PrompterAction::Insert('A')),
			(KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT), PrompterAction::Newline),
			(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT), PrompterAction::Newline),
			(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), PrompterAction::Backspace),
			(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE), PrompterAction::Delete),
			(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL), PrompterAction::Newline),
		];

		// -- Exec & Check
		for (key, expected) in cases {
			assert_eq!(map_prompter_key(&key), Some(expected));
		}
		assert_eq!(map_prompter_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)), None);
		assert_eq!(map_prompter_key(&KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)), None);

		Ok(())
	}

	#[test]
	fn test_core_prompter_keys_is_send_key() -> Result<()> {
		// -- Setup & Fixtures
		let alt_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT);
		let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
		let shift_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT);
		let ctrl_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::CONTROL);
		let ctrl_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL);
		let plain_s = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);

		// -- Exec & Check
		assert!(!is_send_key(&alt_enter));
		assert!(is_send_key(&enter));
		assert!(!is_send_key(&shift_enter));
		assert!(is_send_key(&ctrl_enter));
		assert!(!is_send_key(&ctrl_s));
		assert!(!is_send_key(&plain_s));

		Ok(())
	}
}

// endregion: --- Tests
