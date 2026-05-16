use crate::tui::core::AppState;
use crate::tui::view::style;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::widgets::{Block, Paragraph, Wrap};

pub struct AnswerView;

impl AnswerView {
	pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
		let vertical_chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints([Constraint::Length(0), Constraint::Min(0), Constraint::Length(0)])
			.split(area);
		let content_area = Layout::default()
			.direction(Direction::Horizontal)
			.constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
			.split(vertical_chunks[1])[1];

		f.render_widget(Block::new().style(style::STL_ANSWER), area);
		let content = Paragraph::new(content_text(state))
			.style(style::STL_ANSWER)
			.wrap(Wrap { trim: true });
		f.render_widget(content, content_area);
	}
}

fn content_text(state: &AppState) -> String {
	if let Some(err) = state.last_error() {
		format!("Error: {err}")
	} else if let Some(ans) = state.last_answer() {
		ans.to_string()
	} else {
		"No answer yet. Type a prompt and press Enter.".to_string()
	}
}
