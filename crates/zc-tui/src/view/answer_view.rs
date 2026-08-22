use crate::core::TuiState;
use crate::core::types::ScrollIden;
use crate::view::{style, tblock};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

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

		let lines = build_content_lines(state, content_area.width);
		let line_count = lines.len();
		let scroll = state.clamp_scroll(ScrollIden::AnswerContent, line_count);

		f.render_widget(Block::new().style(style::STL_ANSWER), area);
		let content = Paragraph::new(lines)
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

pub fn build_content_lines(state: &TuiState, content_width: u16) -> Vec<Line<'static>> {
	if let Some(err) = state.last_error() {
		build_error_lines(err, content_width)
	} else if let Some(ans) = state.last_answer() {
		tblock::build_answer_block(ans, content_width)
	} else {
		vec![Line::from(Span::styled(
			"No answer yet. Type a prompt and press Enter.".to_string(),
			style::STL_ANSWER_MUTED,
		))]
	}
}

pub fn build_error_lines(err: &str, content_width: u16) -> Vec<Line<'static>> {
	tblock::build_error_block(err, content_width)
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_view_answer_build_content_lines_precedence() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);

		// -- Exec & Check: default state
		let lines = build_content_lines(&state, 80);
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "No answer yet. Type a prompt and press Enter.");

		// -- Exec & Check: last_answer set
		state.set_last_answer(Some("Model response text".to_string()));
		let lines = build_content_lines(&state, 80);
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "Model response text");

		// -- Exec & Check: last_error takes precedence over last_answer
		state.set_last_error(Some("API request failed".to_string()));
		let lines = build_content_lines(&state, 80);
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Error: API request failed");

		// -- Exec & Check: clearing error restores answer
		state.set_last_error(None);
		let lines = build_content_lines(&state, 80);
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "Model response text");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Execution Error (line 5):\n5 | local x = nil + 1\nattempt to perform arithmetic on a nil value";

		// -- Exec
		let lines = build_error_lines(err, 80);

		// -- Check
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[0].spans.len(), 3);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Lua Execution Error (line 5):");
		assert_eq!(lines[1].spans[0].content, "▌ ");
		assert_eq!(lines[1].spans[1].content, "5 | local x = nil + 1");
		assert_eq!(lines[2].spans[0].content, "▌ ");
		assert_eq!(lines[2].spans[1].content, "attempt to perform arithmetic on a nil value");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_with_stack_trace() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Error (line 3): runtime error\n\n```lua\n3 | error('boom')\n```\n\nStack traceback:\n\tin function 'foo'\n\tin main chunk";

		// -- Exec
		let lines = build_error_lines(err, 80);

		// -- Check
		assert!(!lines.is_empty());
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Lua Error (line 3): runtime error");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_aiprog_stack_trace() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Error (line 12): division by zero\n```lua\n12 | local val = 1 / 0\n```\nStack Trace:\n\t[C]: in function 'error'\n\tscript.lua:12: in main chunk";

		// -- Exec
		let lines = build_error_lines(err, 80);

		// -- Check
		assert_eq!(lines.len(), 7);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Lua Error (line 12): division by zero");
		assert_eq!(lines[1].spans[0].content, "▌ ");
		assert_eq!(lines[1].spans[1].content, "```lua");
		assert_eq!(lines[2].spans[0].content, "▌ ");
		assert_eq!(lines[2].spans[1].content, "12 | local val = 1 / 0");
		assert_eq!(lines[3].spans[0].content, "▌ ");
		assert_eq!(lines[3].spans[1].content, "```");
		assert_eq!(lines[4].spans[0].content, "▌ ");
		assert_eq!(lines[4].spans[1].content, "Stack Trace:");
		assert_eq!(lines[5].spans[0].content, "▌ ");
		assert_eq!(lines[5].spans[1].content, "    [C]: in function 'error'");
		assert_eq!(lines[6].spans[0].content, "▌ ");
		assert_eq!(lines[6].spans[1].content, "    script.lua:12: in main chunk");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_generic_single_line() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Network connection failed";

		// -- Exec
		let lines = build_error_lines(err, 80);

		// -- Check
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans.len(), 3);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Error: Network connection failed");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_empty_input() -> Result<()> {
		// -- Setup & Fixtures
		let err = "";

		// -- Exec
		let lines = build_error_lines(err, 80);

		// -- Check
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans.len(), 3);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Error: Unknown error");

		Ok(())
	}
}

// endregion: --- Tests
