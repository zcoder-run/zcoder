use crate::core::TuiState;
use crate::core::types::ScrollIden;
use crate::view::style;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
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
		let lines = build_content_lines(state);
		let content = Paragraph::new(lines)
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
		if err.starts_with("Error:") || err.starts_with("Lua Execution Error") || err.starts_with("Lua Error") {
			err.to_string()
		} else {
			format!("Error: {err}")
		}
	} else if let Some(ans) = state.last_answer() {
		ans.to_string()
	} else {
		"No answer yet. Type a prompt and press Enter.".to_string()
	}
}

pub fn build_content_lines(state: &TuiState) -> Vec<Line<'static>> {
	if let Some(err) = state.last_error() {
		build_error_lines(err)
	} else if let Some(ans) = state.last_answer() {
		ans.lines()
			.map(|l| Line::from(Span::styled(l.to_string(), style::STL_ANSWER)))
			.collect()
	} else {
		vec![Line::from(Span::styled(
			"No answer yet. Type a prompt and press Enter.".to_string(),
			style::STL_ANSWER_MUTED,
		))]
	}
}

pub fn build_error_lines(err: &str) -> Vec<Line<'static>> {
	let mut lines = Vec::new();
	let mut in_stack_trace = false;

	for (idx, raw_line) in err.lines().enumerate() {
		let line = raw_line.to_string();
		let trimmed = line.trim();

		if idx == 0 {
			let prefix = if trimmed.starts_with("Error:")
				|| trimmed.starts_with("Lua Execution Error")
				|| trimmed.starts_with("Lua Error")
			{
				""
			} else {
				"Error: "
			};
			lines.push(Line::from(vec![
				Span::styled("✘ ", style::STL_ANSWER_ERR),
				Span::styled(format!("{prefix}{line}"), style::STL_ANSWER_ERR_HDR),
			]));
		} else if trimmed.starts_with("Stack traceback:")
			|| trimmed.starts_with("stack traceback:")
			|| trimmed.starts_with("Stack Trace:")
			|| trimmed.starts_with("stack trace:")
		{
			in_stack_trace = true;
			lines.push(Line::from(Span::styled(line, style::STL_ANSWER_ERR_HDR)));
		} else if in_stack_trace
			|| trimmed.starts_with("```")
			|| trimmed.starts_with("-->")
			|| trimmed.contains(" | ")
			|| (trimmed.ends_with('|') && trimmed.chars().all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '|'))
		{
			lines.push(Line::from(Span::styled(line, style::STL_ANSWER_ERR_CODE)));
		} else if trimmed.is_empty() {
			lines.push(Line::from(Span::styled("", style::STL_ANSWER)));
		} else {
			lines.push(Line::from(Span::styled(line, style::STL_ANSWER_ERR_BODY)));
		}
	}

	if lines.is_empty() {
		lines.push(Line::from(vec![
			Span::styled("✘ ", style::STL_ANSWER_ERR),
			Span::styled("Error: Unknown error", style::STL_ANSWER_ERR_HDR),
		]));
	}

	lines
}

fn calculate_line_count(text: &str, width: u16) -> usize {
	if width == 0 || text.is_empty() {
		return 1;
	}
	let width = width as usize;
	let mut count = 0;
	for line in text.lines() {
		let line_len = line.chars().count();
		let wrapped = if line_len == 0 { 1 } else { line_len.div_ceil(width) };
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
		assert_eq!(content_text(&state), "No answer yet. Type a prompt and press Enter.");

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

	#[test]
	fn test_view_answer_build_error_lines() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Execution Error (line 5):\n5 | local x = nil + 1\nattempt to perform arithmetic on a nil value";

		// -- Exec
		let lines = build_error_lines(err);

		// -- Check
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[0].spans.len(), 2);
		assert_eq!(lines[0].spans[0].content, "✘ ");
		assert_eq!(lines[0].spans[1].content, "Lua Execution Error (line 5):");
		assert_eq!(lines[1].spans[0].content, "5 | local x = nil + 1");
		assert_eq!(lines[2].spans[0].content, "attempt to perform arithmetic on a nil value");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_with_stack_trace() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Error (line 3): runtime error\n\n```lua\n3 | error('boom')\n```\n\nStack traceback:\n\tin function 'foo'\n\tin main chunk";

		// -- Exec
		let lines = build_error_lines(err);

		// -- Check
		assert!(!lines.is_empty());
		assert_eq!(lines[0].spans[0].content, "✘ ");
		assert_eq!(lines[0].spans[1].content, "Lua Error (line 3): runtime error");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_aiprog_stack_trace() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Error (line 12): division by zero\n```lua\n12 | local val = 1 / 0\n```\nStack Trace:\n\t[C]: in function 'error'\n\tscript.lua:12: in main chunk";

		// -- Exec
		let lines = build_error_lines(err);

		// -- Check
		assert_eq!(lines.len(), 7);
		assert_eq!(lines[0].spans[0].content, "✘ ");
		assert_eq!(lines[0].spans[1].content, "Lua Error (line 12): division by zero");
		assert_eq!(lines[1].spans[0].content, "```lua");
		assert_eq!(lines[2].spans[0].content, "12 | local val = 1 / 0");
		assert_eq!(lines[3].spans[0].content, "```");
		assert_eq!(lines[4].spans[0].content, "Stack Trace:");
		assert_eq!(lines[5].spans[0].content, "\t[C]: in function 'error'");
		assert_eq!(lines[6].spans[0].content, "\tscript.lua:12: in main chunk");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_generic_single_line() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Network connection failed";

		// -- Exec
		let lines = build_error_lines(err);

		// -- Check
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "✘ ");
		assert_eq!(lines[0].spans[1].content, "Error: Network connection failed");

		Ok(())
	}

	#[test]
	fn test_view_answer_build_error_lines_empty_input() -> Result<()> {
		// -- Setup & Fixtures
		let err = "";

		// -- Exec
		let lines = build_error_lines(err);

		// -- Check
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "✘ ");
		assert_eq!(lines[0].spans[1].content, "Error: Unknown error");

		Ok(())
	}
}

// endregion: --- Tests
