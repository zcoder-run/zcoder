use crate::tui::AppState;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn render(f: &mut Frame, state: &AppState) {
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(3), // Header
			Constraint::Min(0),    // Content
			Constraint::Length(1), // Status
			Constraint::Length(3), // Input
			Constraint::Length(1), // Footer
		])
		.split(f.area());

	// -- Header
	let header = Paragraph::new(" zcoder ")
		.block(Block::default().borders(Borders::ALL))
		.style(Style::default().fg(Color::Cyan));
	f.render_widget(header, chunks[0]);

	// -- Content
	let content_text = if let Some(err) = &state.last_error() {
		format!("Error: {err}")
	} else if let Some(ans) = &state.last_answer() {
		ans.to_string()
	} else {
		"No answer yet. Type a prompt and press Enter.".to_string()
	};

	let content = Paragraph::new(content_text)
		.block(Block::default().borders(Borders::ALL).title(" AI Answer "))
		.wrap(Wrap { trim: true });
	f.render_widget(content, chunks[1]);

	// -- Status
	let status_style = if state.last_error().is_some() {
		Style::default().fg(Color::Red)
	} else if state.is_waiting() {
		Style::default().fg(Color::Yellow)
	} else {
		Style::default().fg(Color::Green)
	};
	let status = Paragraph::new(format!(" Status: {} ", state.status())).style(status_style);
	f.render_widget(status, chunks[2]);

	// -- Input
	let input_style = if state.is_waiting() {
		Style::default().fg(Color::DarkGray)
	} else {
		Style::default()
	};
	let input = Paragraph::new(state.input())
		.block(Block::default().borders(Borders::ALL).title(" Prompt (/q to quit) "))
		.style(input_style);
	f.render_widget(input, chunks[3]);

	// -- Footer
	let footer = Paragraph::new(" [Enter] Send  |  [/q] Quit  |  [Ctrl-c] Quit ");
	f.render_widget(footer, chunks[4]);
}
