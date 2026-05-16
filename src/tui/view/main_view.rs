use crate::tui::core::AppState;
use crate::tui::view::{AnswerView, FooterView, PromptView, StatusView, style};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Block;

pub fn render(f: &mut Frame, state: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Min(0),    // Content
			Constraint::Length(2), // Status
			Constraint::Length(3), // Input
			Constraint::Length(1), // Footer
		])
		.split(f.area());

	f.render_widget(Block::new().style(style::STL_BKG), f.area());

	// -- Content
	AnswerView::render(f, chunks[0], state);

	// -- Status
	StatusView::render(f, chunks[1], state);

	// -- Input
	PromptView::render(f, chunks[2], state);

	// -- Footer
	FooterView::render(f, chunks[3], state);
}
