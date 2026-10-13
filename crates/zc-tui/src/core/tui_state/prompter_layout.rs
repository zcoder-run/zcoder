use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// A visual row, as byte offsets into the source text. `end` excludes the `\n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualRow {
	pub start: usize,
	pub end: usize,
}

/// Wraps the text into visual rows of at most `width` columns, breaking on grapheme boundaries.
/// Every logical line yields at least one row, and at least one row is always returned.
pub fn wrap_rows(text: &str, width: u16) -> Vec<VisualRow> {
	let width = usize::from(width.max(1));
	let mut rows = Vec::new();
	let mut line_start = 0;

	for line in text.split('\n') {
		let line_end = line_start + line.len();
		let mut row_start = line_start;
		let mut col = 0;

		for (idx, grapheme) in line.grapheme_indices(true) {
			let w = UnicodeWidthStr::width(grapheme);
			if col + w > width && col > 0 {
				rows.push(VisualRow { start: row_start, end: line_start + idx });
				row_start = line_start + idx;
				col = 0;
			}
			col += w;
		}

		rows.push(VisualRow { start: row_start, end: line_end });
		line_start = line_end + 1;
	}

	rows
}

/// Returns the (row, column) of the cursor, where the column is the display width from the row start.
/// A cursor at a soft wrap boundary belongs to the next row, except at the end of the text.
pub fn cursor_row_col(text: &str, rows: &[VisualRow], cursor: usize) -> (usize, u16) {
	let last = rows.len().saturating_sub(1);
	let row_idx = rows
		.iter()
		.enumerate()
		.find(|&(i, r)| cursor < r.end || (cursor == r.end && (i == last || rows[i + 1].start != r.end)))
		.map(|(i, _)| i)
		.unwrap_or(last);

	let Some(row) = rows.get(row_idx) else {
		return (0, 0);
	};
	let prefix = text.get(row.start..cursor).unwrap_or_default();
	let col = UnicodeWidthStr::width(prefix);

	(row_idx, u16::try_from(col).unwrap_or(u16::MAX))
}

/// Returns the byte offset closest to the display column `col` within the given row.
/// On a soft-wrapped row, the result never lands on the row end, which belongs to the next row.
pub fn offset_for_row_col(text: &str, rows: &[VisualRow], row: usize, col: u16) -> usize {
	let row_idx = row.min(rows.len().saturating_sub(1));
	let Some(&r) = rows.get(row_idx) else {
		return 0;
	};
	let soft = rows.get(row_idx + 1).is_some_and(|next| next.start == r.end);
	let max_width = usize::from(col);
	let line = text.get(r.start..r.end).unwrap_or_default();

	let mut best = r.start;
	let mut acc = 0;
	for (idx, grapheme) in line.grapheme_indices(true) {
		let w = UnicodeWidthStr::width(grapheme);
		if acc + w > max_width {
			break;
		}
		acc += w;
		best = r.start + idx + grapheme.len();
	}

	if soft && best == r.end && let Some((idx, _)) = line.grapheme_indices(true).next_back() {
		best = r.start + idx;
	}

	best
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

	use super::*;

	#[test]
	fn test_prompter_layout_wrap_rows_lossless() -> Result<()> {
		// -- Setup & Fixtures
		let texts = ["", "hello world", "a\n\nb\n", "中文字符 mixed", "e\u{301}x\n"];
		let widths = [1, 2, 3, 5, 80];

		// -- Exec & Check
		for text in texts {
			for width in widths {
				let rows = wrap_rows(text, width);
				assert_eq!(join_rows(text, &rows), text);
			}
		}

		Ok(())
	}

	#[test]
	fn test_prompter_layout_wrap_rows_empty() -> Result<()> {
		// -- Setup & Fixtures
		let text = "";

		// -- Exec
		let rows = wrap_rows(text, 10);

		// -- Check
		assert_eq!(rows, [vr(0, 0)]);

		Ok(())
	}

	#[test]
	fn test_prompter_layout_wrap_rows_trailing_newline() -> Result<()> {
		// -- Setup & Fixtures
		let text = "a\n";

		// -- Exec
		let rows = wrap_rows(text, 10);

		// -- Check
		assert_eq!(rows, [vr(0, 1), vr(2, 2)]);

		Ok(())
	}

	#[test]
	fn test_prompter_layout_wrap_rows_exact_width() -> Result<()> {
		// -- Setup & Fixtures
		let text = "abcd";

		// -- Exec
		let rows = wrap_rows(text, 2);

		// -- Check
		assert_eq!(rows, [vr(0, 2), vr(2, 4)]);

		Ok(())
	}

	#[test]
	fn test_prompter_layout_wrap_rows_wide_char_at_last_column() -> Result<()> {
		// -- Setup & Fixtures
		let text = "ab中";

		// -- Exec
		let rows = wrap_rows(text, 3);

		// -- Check
		assert_eq!(rows, [vr(0, 2), vr(2, 5)]);

		Ok(())
	}

	#[test]
	fn test_prompter_layout_wrap_rows_zero_width() -> Result<()> {
		// -- Setup & Fixtures
		let text = "abc";

		// -- Exec
		let rows = wrap_rows(text, 0);

		// -- Check
		assert_eq!(rows, [vr(0, 1), vr(1, 2), vr(2, 3)]);

		Ok(())
	}

	#[test]
	fn test_prompter_layout_cursor_row_col_soft_boundary() -> Result<()> {
		// -- Setup & Fixtures
		let text = "abcd";
		let rows = wrap_rows(text, 2);

		// -- Exec & Check
		assert_eq!(cursor_row_col(text, &rows, 2), (1, 0));
		assert_eq!(cursor_row_col(text, &rows, 4), (1, 2));

		Ok(())
	}

	#[test]
	fn test_prompter_layout_cursor_row_col_hard_newline() -> Result<()> {
		// -- Setup & Fixtures
		let text = "ab\ncd";
		let rows = wrap_rows(text, 10);

		// -- Exec & Check
		assert_eq!(cursor_row_col(text, &rows, 2), (0, 2));
		assert_eq!(cursor_row_col(text, &rows, 3), (1, 0));

		Ok(())
	}

	#[test]
	fn test_prompter_layout_cursor_row_col_end_of_text() -> Result<()> {
		// -- Setup & Fixtures
		let text = "ab";
		let rows = wrap_rows(text, 10);

		// -- Exec
		let result = cursor_row_col(text, &rows, 2);

		// -- Check
		assert_eq!(result, (0, 2));

		Ok(())
	}

	#[test]
	fn test_prompter_layout_offset_for_row_col_soft_past_end() -> Result<()> {
		// -- Setup & Fixtures
		let text = "abcd";
		let rows = wrap_rows(text, 2);

		// -- Exec & Check
		assert_eq!(offset_for_row_col(text, &rows, 0, 5), 1);
		assert_eq!(offset_for_row_col(text, &rows, 1, 5), 4);

		Ok(())
	}

	#[test]
	fn test_prompter_layout_offset_for_row_col_hard_row() -> Result<()> {
		// -- Setup & Fixtures
		let text = "ab\ncd";
		let rows = wrap_rows(text, 10);

		// -- Exec & Check
		assert_eq!(offset_for_row_col(text, &rows, 0, 10), 2);
		assert_eq!(offset_for_row_col(text, &rows, 9, 1), 4);

		Ok(())
	}

	// -- Test Support

	fn vr(start: usize, end: usize) -> VisualRow {
		VisualRow { start, end }
	}

	fn join_rows(text: &str, rows: &[VisualRow]) -> String {
		let mut out = String::new();
		for (i, row) in rows.iter().enumerate() {
			if i > 0 && row.start == rows[i - 1].end + 1 {
				out.push('\n');
			}
			out.push_str(&text[row.start..row.end]);
		}
		out
	}
}

// endregion: --- Tests
