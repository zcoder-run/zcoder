use crate::tui::core::AppState;
use crate::tui::view::style;
use ratatui::Frame;
use ratatui::layout::{Margin, Position, Rect};
use ratatui::widgets::{Block, Borders, Paragraph};

pub struct PromptView;

impl PromptView {
	pub fn render(f: &mut Frame, area: Rect, state: &AppState) {
		let input_area = area.inner(Margin {
			horizontal: 1,
			vertical: 0,
		});
		let input_style = style::style_input(state.is_waiting());
		f.render_widget(Block::new().style(style::STL_INPUT), area);
		let input_text = format!("> {}", state.input());
		let input = Paragraph::new(input_text)
			.block(Block::default().borders(Borders::TOP | Borders::BOTTOM).style(style::STL_INPUT_BORDER))
			.style(input_style);
		f.render_widget(input, input_area);

		if !state.is_waiting() && input_area.width > 0 && input_area.height > 1 {
			let cursor_x = input_area
				.x
				.saturating_add(2)
				.saturating_add(state.input().chars().count() as u16)
				.min(input_area.x.saturating_add(input_area.width.saturating_sub(1)));
			let cursor_y = input_area
				.y
				.saturating_add(1)
				.min(input_area.y.saturating_add(input_area.height.saturating_sub(1)));
			f.set_cursor_position(Position::new(cursor_x, cursor_y));
		}
	}
}
