use crate::tui::core::AppState;
use crate::tui::view::style;
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::widgets::Paragraph;

pub struct FooterView;

impl FooterView {
	pub fn render(f: &mut Frame, area: Rect, _state: &AppState) {
		let footer = Paragraph::new(" [Enter] Send  |  [/q] Quit  |  [Ctrl-c] Quit ")
			.style(style::STL_FOOTER);
		f.render_widget(
			footer,
			area.inner(Margin {
				horizontal: 1,
				vertical: 0,
			}),
		);
	}
}
