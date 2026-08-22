use crate::core::TuiState;
use crate::view::style;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Margin, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub struct FooterView;

impl FooterView {
	pub fn render(f: &mut Frame, area: Rect, state: &TuiState) {
		let footer_area = area.inner(Margin {
			horizontal: 1,
			vertical: 0,
		});

		if state.show_sys_states() {
			let sys_spans = vec![
				Span::styled("Mem:", style::STL_SYS_STAT_LBL),
				Span::styled(" ", style::STL_SYS_STAT_VAL),
				Span::styled(state.memory_fmt(), style::STL_SYS_STAT_VAL),
				Span::raw("   "),
				Span::styled("DB:", style::STL_SYS_STAT_LBL),
				Span::styled(" ", style::STL_SYS_STAT_VAL),
				Span::styled(state.db_memory_fmt(), style::STL_SYS_STAT_VAL),
				Span::raw(" "),
			];
			let metric_width = sys_spans.iter().map(|s| s.width() as u16).sum();

			let chunks = Layout::default()
				.direction(Direction::Horizontal)
				.constraints([Constraint::Min(0), Constraint::Length(metric_width)])
				.split(footer_area);

			let footer = Paragraph::new(" [Enter] Send  |  [/q] Quit  |  [Ctrl-c] Quit ").style(style::STL_FOOTER);
			f.render_widget(footer, chunks[0]);

			let metric_par = Paragraph::new(Line::from(sys_spans));
			f.render_widget(metric_par, chunks[1]);
		} else {
			let footer = Paragraph::new(" [Enter] Send  |  [/q] Quit  |  [Ctrl-c] Quit ").style(style::STL_FOOTER);
			f.render_widget(footer, footer_area);
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use ratatui::Terminal;
	use ratatui::backend::TestBackend;

	#[test]
	fn test_footer_view_render_with_sys_states() -> Result<()> {
		let backend = TestBackend::new(80, 1);
		let mut terminal = Terminal::new(backend)?;
		let mut state = TuiState::new(None);
		state.set_show_sys_states(true);
		state.set_db_memory(247_459);

		terminal.draw(|f| {
			FooterView::render(f, f.area(), &state);
		})?;

		let buffer = terminal.backend().buffer();
		let content: String = (0..buffer.area.width)
			.filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
			.collect();

		assert!(content.contains("Mem:"));
		assert!(content.contains("DB:"));
		assert!(content.contains("241.66 KB"));

		Ok(())
	}

	#[test]
	fn test_footer_view_render_without_sys_states() -> Result<()> {
		let backend = TestBackend::new(80, 1);
		let mut terminal = Terminal::new(backend)?;
		let state = TuiState::new(None);

		terminal.draw(|f| {
			FooterView::render(f, f.area(), &state);
		})?;

		let buffer = terminal.backend().buffer();
		let content: String = (0..buffer.area.width)
			.filter_map(|x| buffer.cell((x, 0)).map(|c| c.symbol().to_string()))
			.collect();

		assert!(content.contains("[Enter] Send"));
		assert!(!content.contains("Mem:"));
		assert!(!content.contains("DB:"));

		Ok(())
	}
}

// endregion: --- Tests
