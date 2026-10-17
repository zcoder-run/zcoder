use crate::core::{PROMPTER_MAX_ROWS, TuiState};
use crate::view::comp::{PROMPTER_MARKER_WIDTH, build_prompter_lines};
use crate::view::style;
use ratatui::Frame;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};

const INPUT_MARGIN_H: u16 = 1;

pub struct PromptView;

impl PromptView {
	/// Width left for the prompt text, after margins and the marker.
	pub fn text_width(total_width: u16) -> u16 {
		total_width.saturating_sub(INPUT_MARGIN_H * 2).saturating_sub(PROMPTER_MARKER_WIDTH)
	}

	/// Height of the bordered prompt area: 1 to `PROMPTER_MAX_ROWS` text rows plus borders.
	pub fn height(state: &TuiState, total_width: u16) -> u16 {
		let rows = state
			.prompter()
			.row_count_for(Self::text_width(total_width))
			.clamp(1, usize::from(PROMPTER_MAX_ROWS));
		u16::try_from(rows).unwrap_or(PROMPTER_MAX_ROWS).saturating_add(2)
	}

	/// Prompter help, shown right aligned in the status bar.
	pub fn help_text() -> &'static str {
		" Enter to send  |  Ctrl-j newline  |  /q to quit "
	}

	pub fn render(f: &mut Frame, area: Rect, state: &mut TuiState) {
		let input_area = area.inner(Margin {
			horizontal: INPUT_MARGIN_H,
			vertical: 0,
		});
		let input_style = style::style_input(state.is_waiting());
		f.render_widget(Block::new().style(style::STL_INPUT), area);

		state.prompter_mut().set_viewport_width(Self::text_width(area.width));
		state.prompter_mut().ensure_cursor_visible();
		let prompter_render = build_prompter_lines(state.prompter(), state.is_waiting());

		let input = Paragraph::new(prompter_render.lines)
			.block(
				Block::default()
					.borders(Borders::TOP | Borders::BOTTOM)
					.style(style::STL_INPUT_BORDER),
			)
			.style(input_style);
		f.render_widget(input, input_area);

		if input_area.width > 0 && input_area.height > 2 {
			let (cursor_col, cursor_row) = prompter_render.cursor;
			let cursor_x = input_area
				.x
				.saturating_add(cursor_col)
				.min(input_area.x.saturating_add(input_area.width.saturating_sub(1)));
			let cursor_y = input_area
				.y
				.saturating_add(1)
				.saturating_add(cursor_row)
				.min(input_area.y.saturating_add(input_area.height.saturating_sub(2)));
			f.set_cursor_position(Position::new(cursor_x, cursor_y));
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_view_prompt_view_height_one_row() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.prompter_mut().insert_str("hello");

		// -- Exec
		let height = PromptView::height(&state, 80);

		// -- Check
		assert_eq!(height, 3);

		Ok(())
	}

	#[test]
	fn test_view_prompt_view_height_max_rows() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.prompter_mut().insert_str("a\nb\nc\nd\ne");

		// -- Exec
		let height = PromptView::height(&state, 80);

		// -- Check
		assert_eq!(height, 5);

		Ok(())
	}
}

// endregion: --- Tests
