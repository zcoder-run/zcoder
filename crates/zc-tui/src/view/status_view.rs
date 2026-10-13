use crate::core::TuiState;
use crate::view::prompt_view::PromptView;
use crate::view::style;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

/// Minimum width kept for the status message before the help is shown.
const STATUS_MIN_WIDTH: u16 = 20;

pub struct StatusView;

impl StatusView {
	pub fn render(f: &mut Frame, area: Rect, state: &TuiState) {
		let status_style = style::style_status(state.last_error().is_some(), state.is_waiting());
		let status = Paragraph::new(format!(" Status: {} ", state.status())).style(status_style);
		let status_area = if area.height > 1 {
			Rect {
				y: area.y + 1,
				height: 1,
				..area
			}
		} else {
			area
		};
		let status_area = status_area.inner(Margin {
			horizontal: 1,
			vertical: 0,
		});

		let help = PromptView::help_text();
		let help_w = u16::try_from(help.width()).unwrap_or(u16::MAX);
		if status_area.width >= help_w.saturating_add(STATUS_MIN_WIDTH) {
			let chunks = Layout::default()
				.direction(Direction::Horizontal)
				.constraints([Constraint::Min(0), Constraint::Length(help_w)])
				.split(status_area);
			f.render_widget(status, chunks[0]);
			f.render_widget(Paragraph::new(help).style(style::STL_STATUS_HELP), chunks[1]);
		} else {
			f.render_widget(status, status_area);
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;
	use ratatui::Terminal;
	use ratatui::backend::TestBackend;

	fn render_row(width: u16) -> Result<String> {
		let backend = TestBackend::new(width, 2);
		let mut terminal = Terminal::new(backend)?;
		let state = TuiState::new(None);

		terminal.draw(|f| {
			StatusView::render(f, f.area(), &state);
		})?;

		let buffer = terminal.backend().buffer();
		let content: String = (0..buffer.area.width)
			.filter_map(|x| buffer.cell((x, 1)).map(|c| c.symbol().to_string()))
			.collect();
		Ok(content)
	}

	#[test]
	fn test_view_status_view_render_with_help() -> Result<()> {
		// -- Setup & Fixtures & Exec
		let content = render_row(120)?;

		// -- Check
		assert!(content.contains("Status: Idle"));
		assert!(content.contains("Enter to send"));
		assert!(content.trim_end().ends_with("/q to quit"));

		Ok(())
	}

	#[test]
	fn test_view_status_view_render_narrow_hides_help() -> Result<()> {
		// -- Setup & Fixtures & Exec
		let content = render_row(30)?;

		// -- Check
		assert!(content.contains("Status: Idle"));
		assert!(!content.contains("to send"));

		Ok(())
	}
}

// endregion: --- Tests
