use crate::core::{PROMPTER_MAX_ROWS, PrompterState};
use crate::view::style;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use std::ops::Range;

pub struct PrompterRender {
	pub lines: Vec<Line<'static>>,
	pub cursor: (u16, u16),
}

pub const PROMPTER_MARKER: &str = "> ";
pub const PROMPTER_MARKER_WIDTH: u16 = 2;

pub fn build_prompter_lines(prompter: &PrompterState, waiting: bool) -> PrompterRender {
	let text_style = style::style_input(waiting);
	let text = prompter.text();
	let selection = prompter.selection();
	let rows = prompter.rows();
	let scroll = usize::from(prompter.scroll_y());
	let (cursor_row, cursor_col) = prompter.cursor_row_col();

	let lines: Vec<Line<'static>> = rows
		.iter()
		.enumerate()
		.skip(scroll)
		.take(usize::from(PROMPTER_MAX_ROWS))
		.map(|(idx, row)| {
			let marker = if idx == 0 {
				PROMPTER_MARKER.to_string()
			} else {
				" ".repeat(usize::from(PROMPTER_MARKER_WIDTH))
			};
			let mut spans = vec![Span::styled(marker, text_style)];
			spans.extend(row_spans(text, row.start, row.end, selection.as_ref(), text_style));
			Line::from(spans)
		})
		.collect();

	let cursor_y = u16::try_from(cursor_row.saturating_sub(scroll)).unwrap_or(u16::MAX);
	let cursor_x = PROMPTER_MARKER_WIDTH.saturating_add(cursor_col);

	PrompterRender {
		lines,
		cursor: (cursor_x, cursor_y),
	}
}

// region:    --- Support

fn row_spans(
	text: &str,
	start: usize,
	end: usize,
	selection: Option<&Range<usize>>,
	text_style: Style,
) -> Vec<Span<'static>> {
	let Some(sel) = selection else {
		return vec![Span::styled(slice_text(text, start, end), text_style)];
	};

	let sel_start = sel.start.clamp(start, end);
	let sel_end = sel.end.clamp(start, end);
	if sel_start >= sel_end {
		return vec![Span::styled(slice_text(text, start, end), text_style)];
	}

	let mut spans = Vec::with_capacity(3);
	if sel_start > start {
		spans.push(Span::styled(slice_text(text, start, sel_start), text_style));
	}
	spans.push(Span::styled(
		slice_text(text, sel_start, sel_end),
		style::STL_INPUT_SELECTION,
	));
	if sel_end < end {
		spans.push(Span::styled(slice_text(text, sel_end, end), text_style));
	}
	spans
}

fn slice_text(text: &str, start: usize, end: usize) -> String {
	text.get(start..end).unwrap_or_default().to_string()
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_view_comp_prompter_build_lines_marker_and_indent() -> Result<()> {
		// -- Setup & Fixtures
		let mut prompter = PrompterState::from_text("ab\ncd");
		prompter.set_viewport_width(20);

		// -- Exec
		let render = build_prompter_lines(&prompter, false);

		// -- Check
		assert_eq!(render.lines.len(), 2);
		let first = render.lines.first().ok_or("should have first line")?;
		let second = render.lines.get(1).ok_or("should have second line")?;
		let first_marker = first.spans.first().ok_or("should have first marker")?;
		let second_marker = second.spans.first().ok_or("should have second marker")?;
		assert_eq!(first_marker.content, "> ");
		assert_eq!(second_marker.content, "  ");

		Ok(())
	}

	#[test]
	fn test_view_comp_prompter_build_lines_selection_split() -> Result<()> {
		// -- Setup & Fixtures
		let mut prompter = PrompterState::from_text("hello world");
		prompter.set_viewport_width(20);
		prompter.move_text_start(false);
		prompter.move_right(false);
		prompter.move_right(false);
		prompter.move_right(true);
		prompter.move_right(true);
		prompter.move_right(true);

		// -- Exec
		let render = build_prompter_lines(&prompter, false);

		// -- Check
		let first = render.lines.first().ok_or("should have first line")?;
		assert_eq!(first.spans.len(), 4);
		let selected = first.spans.get(2).ok_or("should have selected span")?;
		assert_eq!(selected.content, "llo");

		Ok(())
	}

	#[test]
	fn test_view_comp_prompter_build_lines_cursor_wrapped_row() -> Result<()> {
		// -- Setup & Fixtures
		let mut prompter = PrompterState::from_text("abcdef");
		prompter.set_viewport_width(2);

		// -- Exec
		let render = build_prompter_lines(&prompter, false);

		// -- Check
		assert_eq!(render.cursor, (4, 2));

		Ok(())
	}

	#[test]
	fn test_view_comp_prompter_build_lines_max_three_rows() -> Result<()> {
		// -- Setup & Fixtures
		let mut prompter = PrompterState::from_text("a\nb\nc\nd\ne");
		prompter.set_viewport_width(10);

		// -- Exec
		let render = build_prompter_lines(&prompter, false);

		// -- Check
		assert_eq!(render.lines.len(), 3);

		Ok(())
	}
}

// endregion: --- Tests
