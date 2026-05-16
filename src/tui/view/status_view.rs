use crate::tui::core::AppState;
use crate::tui::view::style;
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::widgets::Paragraph;

pub struct StatusView;

impl StatusView {
	pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
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
		f.render_widget(
			status,
			status_area.inner(Margin {
				horizontal: 1,
				vertical: 0,
			}),
		);
	}
}
