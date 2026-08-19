use crate::core::types::ScrollIden;
use crate::core::TuiState;
use crate::view::style;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap};

pub struct AnswerView;

impl AnswerView {
	pub fn render(f: &mut Frame, area: Rect, state: &mut TuiState) {
		let vertical_chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Length(0), Constraint::Min(0), Constraint::Length(0)])
			.split(area);
		let content_area = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
			.split(vertical_chunks[1])[1];

		state.set_scroll_area(ScrollIden::AnswerContent, content_area);

		let text = content_text(state);
		let line_count = calculate_line_count(&text, content_area.width);
		let scroll = state.clamp_scroll(ScrollIden::AnswerContent, line_count);

		f.render_widget(Block::new().style(style::STL_ANSWER), area);
		let content = Paragraph::new(text)
			.style(style::STL_ANSWER)
			.wrap(Wrap { trim: true })
			.scroll((scroll, 0));
		f.render_widget(content, content_area);

		let content_size = line_count.saturating_sub(content_area.height as usize);
		let mut scrollbar_state = ScrollbarState::new(content_size).position(scroll as usize);
		let scrollbar = Scrollbar::default()
			.orientation(ScrollbarOrientation::VerticalRight)
			.begin_symbol(Some("▲"))
			.end_symbol(Some("▼"));
		f.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
	}
}

fn content_text(state: &TuiState) -> String {
	if let Some(err) = state.last_error() {
		format!("Error: {err}")
	} else if let Some(ans) = state.last_answer() {
		ans.to_string()
	} else {
		"No answer yet. Type a prompt and press Enter.".to_string()
	}
}

fn calculate_line_count(text: &str, width: u16) -> usize {
	if width == 0 || text.is_empty() {
		return 1;
	}
	let width = width as usize;
	let mut count = 0;
	for line in text.lines() {
		let line_len = line.chars().count();
		let wrapped = if line_len == 0 {
			1
		} else {
			line_len.div_ceil(width)
		};
		count += wrapped;
	}
	count.max(1)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_view_answer_calculate_line_count_simple() -> Result<()> {
		// -- Setup & Fixtures
		let text = "Hello world";
		let width = 20;

		// -- Exec
		let count = calculate_line_count(text, width);

		// -- Check
		assert_eq!(count, 1);

		Ok(())
	}

	#[test]
	fn test_view_answer_calculate_line_count_wrapping() -> Result<()> {
		// -- Setup & Fixtures
		let text = "This is a longer line of text that wraps";
		let width = 10;

		// -- Exec
		let count = calculate_line_count(text, width);

		// -- Check
		assert_eq!(count, 4);

		Ok(())
	}

	#[test]
	fn test_view_answer_calculate_line_count_multiline() -> Result<()> {
		// -- Setup & Fixtures
		let text = "Line 1\nLine 2\n\nLine 4";
		let width = 50;

		// -- Exec
		let count = calculate_line_count(text, width);

		// -- Check
		assert_eq!(count, 4);

		Ok(())
	}

	#[test]
	fn test_view_answer_calculate_line_count_edge_cases() -> Result<()> {
		// -- Setup & Fixtures & Exec & Check
		assert_eq!(calculate_line_count("", 80), 1);
		assert_eq!(calculate_line_count("test", 0), 1);

		Ok(())
	}

	#[test]
	fn test_view_answer_content_text_precedence() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);

		// -- Exec & Check: default state
		assert_eq!(
			content_text(&state),
			"No answer yet. Type a prompt and press Enter."
		);

		// -- Exec & Check: last_answer set
		state.set_last_answer(Some("Model response text".to_string()));
		assert_eq!(content_text(&state), "Model response text");

		// -- Exec & Check: last_error takes precedence over last_answer
		state.set_last_error(Some("API request failed".to_string()));
		assert_eq!(content_text(&state), "Error: API request failed");

		// -- Exec & Check: clearing error restores answer
		state.set_last_error(None);
		assert_eq!(content_text(&state), "Model response text");

		Ok(())
	}
}

// endregion: --- Tests
