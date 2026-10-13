use super::prompter_layout::{self, VisualRow};
use crate::core::prompter_keys::{PrompterAction, PrompterMotion};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

pub const PROMPTER_MAX_ROWS: u16 = 3;

/// Prompt text buffer with cursor, selection, scroll, and visual row helpers.
#[derive(Debug, Default, Clone)]
pub struct PrompterState {
	text: String,
	cursor: usize,
	anchor: Option<usize>,
	preferred_col: Option<u16>,
	scroll_y: u16,
	viewport_width: u16,
}

/// Constructors and accessors
impl PrompterState {
	pub fn from_text(text: impl Into<String>) -> Self {
		let text = text.into();
		let cursor = text.len();
		Self {
			text,
			cursor,
			..Default::default()
		}
	}

	pub fn text(&self) -> &str {
		&self.text
	}

	#[allow(unused)]
	pub fn cursor(&self) -> usize {
		self.cursor
	}

	#[allow(unused)]
	pub fn is_empty(&self) -> bool {
		self.text.is_empty()
	}

	pub fn scroll_y(&self) -> u16 {
		self.scroll_y
	}

	pub fn selection(&self) -> Option<Range<usize>> {
		let anchor = self.anchor?;
		if anchor == self.cursor {
			None
		} else {
			Some(anchor.min(self.cursor)..anchor.max(self.cursor))
		}
	}

	pub fn selected_text(&self) -> Option<&str> {
		self.selection().and_then(|range| self.text.get(range))
	}
}

/// Edits
impl PrompterState {
	/// Normalizes `\r\n` and `\r` to `\n`, and tabs to 4 spaces.
	pub fn insert_str(&mut self, text: &str) {
		let normalized = text.replace("\r\n", "\n").replace('\r', "\n").replace('\t', "    ");
		self.begin_edit();
		self.text.insert_str(self.cursor, &normalized);
		self.cursor += normalized.len();
	}

	pub fn insert_char(&mut self, c: char) {
		self.begin_edit();
		self.text.insert(self.cursor, c);
		self.cursor += c.len_utf8();
	}

	pub fn insert_newline(&mut self) {
		self.insert_char('\n');
	}

	/// Replaces a `\` right before the cursor with a newline, when nothing is selected.
	/// Returns true when the line continuation was applied.
	pub fn continue_line(&mut self) -> bool {
		let has_backslash = self.text.get(..self.cursor).is_some_and(|prefix| prefix.ends_with('\\'));
		if self.selection().is_some() || !has_backslash {
			return false;
		}
		let start = self.cursor - 1;
		self.begin_edit();
		self.text.replace_range(start..self.cursor, "\n");
		true
	}

	pub fn backspace(&mut self) {
		if self.selection().is_some() {
			self.begin_edit();
		} else if self.cursor > 0 {
			self.begin_edit();
			let start = prev_grapheme_start(&self.text, self.cursor);
			self.text.replace_range(start..self.cursor, "");
			self.cursor = start;
		}
	}

	pub fn delete(&mut self) {
		if self.selection().is_some() {
			self.begin_edit();
		} else if self.cursor < self.text.len() {
			self.begin_edit();
			let end = next_grapheme_end(&self.text, self.cursor);
			self.text.replace_range(self.cursor..end, "");
		}
	}

	/// Removes the selection, if any, and resets the edit state.
	fn begin_edit(&mut self) {
		if let Some(range) = self.selection() {
			let start = range.start;
			self.text.replace_range(range, "");
			self.cursor = start;
		}
		self.anchor = None;
		self.preferred_col = None;
	}
}

/// Movement
impl PrompterState {
	pub fn move_left(&mut self, select: bool) {
		if !select && let Some(range) = self.selection() {
			self.move_to(range.start, false);
		} else {
			let target = prev_grapheme_start(&self.text, self.cursor);
			self.move_to(target, select);
		}
	}

	pub fn move_right(&mut self, select: bool) {
		if !select && let Some(range) = self.selection() {
			self.move_to(range.end, false);
		} else {
			let target = next_grapheme_end(&self.text, self.cursor);
			self.move_to(target, select);
		}
	}

	pub fn move_word_left(&mut self, select: bool) {
		let target = prev_word_start(&self.text, self.cursor);
		self.move_to(target, select);
	}

	pub fn move_word_right(&mut self, select: bool) {
		let target = next_word_end(&self.text, self.cursor);
		self.move_to(target, select);
	}

	/// Moves one visual row up, keeping the preferred column.
	pub fn move_up(&mut self, select: bool) {
		let rows = self.rows();
		let (row, col) = prompter_layout::cursor_row_col(&self.text, &rows, self.cursor);
		if row == 0 {
			self.move_to(0, select);
		} else {
			let col = self.preferred_col.unwrap_or(col);
			self.move_row(&rows, row - 1, col, select);
		}
	}

	/// Moves one visual row down, keeping the preferred column.
	pub fn move_down(&mut self, select: bool) {
		let rows = self.rows();
		let (row, col) = prompter_layout::cursor_row_col(&self.text, &rows, self.cursor);
		if row + 1 >= rows.len() {
			let end = self.text.len();
			self.move_to(end, select);
		} else {
			let col = self.preferred_col.unwrap_or(col);
			self.move_row(&rows, row + 1, col, select);
		}
	}

	pub fn move_row_start(&mut self, select: bool) {
		let rows = self.rows();
		let (row, _) = prompter_layout::cursor_row_col(&self.text, &rows, self.cursor);
		let start = rows.get(row).map_or(0, |r| r.start);
		self.move_to(start, select);
	}

	pub fn move_row_end(&mut self, select: bool) {
		let rows = self.rows();
		let (row, _) = prompter_layout::cursor_row_col(&self.text, &rows, self.cursor);
		let end = prompter_layout::offset_for_row_col(&self.text, &rows, row, u16::MAX);
		self.move_to(end, select);
	}

	pub fn move_text_start(&mut self, select: bool) {
		self.move_to(0, select);
	}

	pub fn move_text_end(&mut self, select: bool) {
		let end = self.text.len();
		self.move_to(end, select);
	}

	fn move_to(&mut self, target: usize, select: bool) {
		self.prepare_move(select);
		self.cursor = target;
		self.preferred_col = None;
	}

	fn move_row(&mut self, rows: &[VisualRow], row: usize, col: u16, select: bool) {
		let target = prompter_layout::offset_for_row_col(&self.text, rows, row, col);
		self.prepare_move(select);
		self.cursor = target;
		self.preferred_col = Some(col);
	}

	/// Starts a selection when extending, and clears it otherwise.
	fn prepare_move(&mut self, select: bool) {
		if select {
			self.anchor = self.anchor.or(Some(self.cursor));
		} else {
			self.anchor = None;
		}
	}
}

/// Key actions
impl PrompterState {
	pub fn apply(&mut self, action: &PrompterAction) {
		match *action {
			PrompterAction::Insert(c) => self.insert_char(c),
			PrompterAction::Newline => self.insert_newline(),
			PrompterAction::Backspace => self.backspace(),
			PrompterAction::Delete => self.delete(),
			PrompterAction::Move { motion, select } => match motion {
				PrompterMotion::Left => self.move_left(select),
				PrompterMotion::Right => self.move_right(select),
				PrompterMotion::WordLeft => self.move_word_left(select),
				PrompterMotion::WordRight => self.move_word_right(select),
				PrompterMotion::Up => self.move_up(select),
				PrompterMotion::Down => self.move_down(select),
				PrompterMotion::RowStart => self.move_row_start(select),
				PrompterMotion::RowEnd => self.move_row_end(select),
				PrompterMotion::TextStart => self.move_text_start(select),
				PrompterMotion::TextEnd => self.move_text_end(select),
			},
			PrompterAction::SelectAll => self.select_all(),
			PrompterAction::ClearSelection => self.clear_selection(),
			// Copy and Paste need the clipboard, which the event handler owns.
			PrompterAction::Copy | PrompterAction::Paste => {}
		}
	}
}

/// Selection and submit support
impl PrompterState {
	pub fn select_all(&mut self) {
		self.anchor = Some(0);
		self.cursor = self.text.len();
		self.preferred_col = None;
	}

	pub fn clear_selection(&mut self) {
		self.anchor = None;
	}

	/// Returns the text and resets every field except the viewport width.
	pub fn take_text(&mut self) -> String {
		self.cursor = 0;
		self.anchor = None;
		self.preferred_col = None;
		self.scroll_y = 0;
		std::mem::take(&mut self.text)
	}

	/// Puts restored text before the current text, separated by a newline.
	pub fn restore_text(&mut self, restored: &str) {
		if self.text.is_empty() {
			self.text = restored.to_string();
			self.cursor = self.text.len();
			self.anchor = None;
		} else {
			let shift = restored.len() + 1;
			self.text = format!("{restored}\n{}", self.text);
			self.cursor += shift;
			self.anchor = self.anchor.map(|a| a + shift);
		}
		self.preferred_col = None;
	}
}

/// Layout and scroll support
impl PrompterState {
	pub fn set_viewport_width(&mut self, width: u16) {
		self.viewport_width = width;
	}

	pub fn rows(&self) -> Vec<VisualRow> {
		prompter_layout::wrap_rows(&self.text, self.effective_width())
	}

	pub fn row_count_for(&self, width: u16) -> usize {
		prompter_layout::wrap_rows(&self.text, width).len()
	}

	pub fn cursor_row_col(&self) -> (usize, u16) {
		prompter_layout::cursor_row_col(&self.text, &self.rows(), self.cursor)
	}

	/// Keeps the cursor row inside the visible window and clamps the scroll.
	pub fn ensure_cursor_visible(&mut self) {
		let rows = self.rows();
		let (row, _) = prompter_layout::cursor_row_col(&self.text, &rows, self.cursor);
		let max_rows = usize::from(PROMPTER_MAX_ROWS);
		let mut scroll = usize::from(self.scroll_y);
		if row < scroll {
			scroll = row;
		} else if row >= scroll + max_rows {
			scroll = row + 1 - max_rows;
		}
		let max_scroll = rows.len().saturating_sub(max_rows);
		self.scroll_y = u16::try_from(scroll.min(max_scroll)).unwrap_or(u16::MAX);
	}

	/// A width of 0 means unbounded, so no soft wrapping happens.
	fn effective_width(&self) -> u16 {
		if self.viewport_width == 0 {
			u16::MAX
		} else {
			self.viewport_width
		}
	}
}

// region:    --- Support

fn is_word_char(c: char) -> bool {
	c.is_alphanumeric() || c == '_'
}

fn prev_grapheme_start(text: &str, idx: usize) -> usize {
	text.get(..idx)
		.and_then(|prefix| prefix.grapheme_indices(true).next_back())
		.map_or(0, |(i, _)| i)
}

fn next_grapheme_end(text: &str, idx: usize) -> usize {
	text.get(idx..)
		.and_then(|suffix| suffix.graphemes(true).next())
		.map_or(text.len(), |g| idx + g.len())
}

fn prev_word_start(text: &str, idx: usize) -> usize {
	let prefix = text.get(..idx).unwrap_or_default();
	let mut chars = prefix.char_indices().rev().peekable();
	let mut pos = idx;
	while let Some(&(i, c)) = chars.peek()
		&& !is_word_char(c)
	{
		pos = i;
		chars.next();
	}
	while let Some(&(i, c)) = chars.peek()
		&& is_word_char(c)
	{
		pos = i;
		chars.next();
	}
	pos
}

fn next_word_end(text: &str, idx: usize) -> usize {
	let suffix = text.get(idx..).unwrap_or_default();
	let mut chars = suffix.char_indices().peekable();
	let mut pos = idx;
	while let Some(&(i, c)) = chars.peek()
		&& !is_word_char(c)
	{
		pos = idx + i + c.len_utf8();
		chars.next();
	}
	while let Some(&(i, c)) = chars.peek()
		&& is_word_char(c)
	{
		pos = idx + i + c.len_utf8();
		chars.next();
	}
	pos
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
#[path = "prompter_state_tests.rs"]
mod tests;

// endregion: --- Tests
