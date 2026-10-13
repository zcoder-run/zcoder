type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;

#[test]
fn test_prompter_state_backspace_delete_graphemes() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("a中e\u{301}");

	// -- Exec & Check
	state.backspace();
	assert_eq!(state.text(), "a中");
	assert_eq!(state.cursor(), 4);

	state.backspace();
	assert_eq!(state.text(), "a");
	assert_eq!(state.cursor(), 1);

	state.move_text_start(false);
	state.delete();
	assert_eq!(state.text(), "");
	assert_eq!(state.cursor(), 0);

	Ok(())
}

#[test]
fn test_prompter_state_insert_replaces_selection() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("hello world");
	state.move_text_start(false);
	for _ in 0..5 {
		state.move_right(true);
	}

	// -- Exec
	state.insert_char('X');

	// -- Check
	assert_eq!(state.text(), "X world");
	assert_eq!(state.cursor(), 1);
	assert_eq!(state.selection(), None);

	Ok(())
}

#[test]
fn test_prompter_state_insert_str_normalizes() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("ab");
	state.move_left(false);

	// -- Exec
	state.insert_str("x\r\ny\rz\tq");

	// -- Check
	assert_eq!(state.text(), "ax\ny\nz    qb");
	assert_eq!(state.cursor(), 11);

	Ok(())
}

#[test]
fn test_prompter_state_move_word_left_right() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("foo bar_baz qux");

	// -- Exec & Check
	state.move_word_left(false);
	assert_eq!(state.cursor(), 12);

	state.move_word_left(false);
	assert_eq!(state.cursor(), 4);

	state.move_word_right(false);
	assert_eq!(state.cursor(), 11);

	state.move_word_right(false);
	assert_eq!(state.cursor(), 15);

	Ok(())
}

#[test]
fn test_prompter_state_move_up_down_keeps_preferred_col() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("abcdefgh\nab\nabcdefgh");
	state.set_viewport_width(4);
	state.move_text_start(false);
	for _ in 0..6 {
		state.move_right(false);
	}

	// -- Exec & Check
	state.move_up(false);
	assert_eq!(state.cursor(), 2);

	state.move_down(false);
	assert_eq!(state.cursor(), 6);

	// Short row clamps the cursor, but the preferred column is kept.
	state.move_down(false);
	assert_eq!(state.cursor(), 11);

	state.move_down(false);
	assert_eq!(state.cursor(), 14);

	Ok(())
}

#[test]
fn test_prompter_state_move_up_first_row_down_last_row() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("abc\ndef");

	// -- Exec & Check
	state.move_up(false);
	assert_eq!(state.cursor(), 3);

	state.move_up(false);
	assert_eq!(state.cursor(), 0);

	state.move_down(false);
	assert_eq!(state.cursor(), 4);

	state.move_down(false);
	assert_eq!(state.cursor(), 7);

	Ok(())
}

#[test]
fn test_prompter_state_left_right_collapse_selection() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("abcdef");

	// -- Exec & Check
	for _ in 0..3 {
		state.move_left(true);
	}
	assert_eq!(state.selection(), Some(3..6));
	assert_eq!(state.selected_text(), Some("def"));

	state.move_left(false);
	assert_eq!(state.cursor(), 3);
	assert_eq!(state.selection(), None);

	for _ in 0..3 {
		state.move_left(true);
	}
	state.move_right(false);
	assert_eq!(state.cursor(), 3);
	assert_eq!(state.selection(), None);

	Ok(())
}

#[test]
fn test_prompter_state_take_and_restore() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("hello");

	// -- Exec & Check
	let taken = state.take_text();
	assert_eq!(taken, "hello");
	assert!(state.is_empty());
	assert_eq!(state.cursor(), 0);

	state.restore_text("draft");
	assert_eq!(state.text(), "draft");
	assert_eq!(state.cursor(), 5);

	state.insert_char('!');
	state.restore_text("prompt");
	assert_eq!(state.text(), "prompt\ndraft!");
	assert_eq!(state.cursor(), 13);

	Ok(())
}

#[test]
fn test_prompter_state_ensure_cursor_visible_scroll() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("aabbccddeeffgg");
	state.set_viewport_width(2);

	// -- Exec & Check
	state.ensure_cursor_visible();
	assert_eq!(state.scroll_y(), 4);

	state.move_text_start(false);
	state.ensure_cursor_visible();
	assert_eq!(state.scroll_y(), 0);

	Ok(())
}

#[test]
fn test_prompter_state_apply_move_select_and_insert() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = PrompterState::from_text("hello");

	// -- Exec
	state.apply(&PrompterAction::Move {
		motion: PrompterMotion::WordLeft,
		select: true,
	});

	// -- Check
	assert_eq!(state.selected_text(), Some("hello"));

	// -- Exec
	state.apply(&PrompterAction::Insert('X'));

	// -- Check
	assert_eq!(state.text(), "X");
	assert_eq!(state.cursor(), 1);

	Ok(())
}
