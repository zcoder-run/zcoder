type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;

#[test]
fn test_view_prompter_view_height_one_row() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = TuiState::new(None);
	state.prompter_mut().insert_str("hello");

	// -- Exec
	let height = PrompterView::height(&state, 80);

	// -- Check
	assert_eq!(height, 3);

	Ok(())
}

#[test]
fn test_view_prompter_view_height_max_rows() -> Result<()> {
	// -- Setup & Fixtures
	let mut state = TuiState::new(None);
	state.prompter_mut().insert_str("a\nb\nc\nd\ne");

	// -- Exec
	let height = PrompterView::height(&state, 80);

	// -- Check
	assert_eq!(height, 5);

	Ok(())
}
