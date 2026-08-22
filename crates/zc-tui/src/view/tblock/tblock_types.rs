use crate::view::style::{
	STL_BAR_ANSWER, STL_BAR_ERR, STL_BAR_PROMPT, STL_BAR_RUNNING, STL_TBLOCK_ANSWER, STL_TBLOCK_PROMPT,
};
use ratatui::style::Style;

// region:    --- Types

/// The visual kind and role of a TBlock.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TBlockKind {
	Prompt,
	Answer,
	Error,
	Running,
}

// endregion: --- Types

impl TBlockKind {
	/// Returns the indicator bar style for this block kind.
	pub fn bar_style(&self) -> Style {
		match self {
			Self::Prompt => STL_BAR_PROMPT,
			Self::Answer => STL_BAR_ANSWER,
			Self::Error => STL_BAR_ERR,
			Self::Running => STL_BAR_RUNNING,
		}
	}

	/// Returns the default body text style for this block kind.
	#[allow(dead_code)]
	pub fn content_style(&self) -> Style {
		match self {
			Self::Prompt => STL_TBLOCK_PROMPT,
			Self::Answer => STL_TBLOCK_ANSWER,
			Self::Error => STL_BAR_ERR,
			Self::Running => STL_TBLOCK_ANSWER,
		}
	}

	/// Returns the vertical bar glyph string used for the indicator bar.
	pub fn bar_glyph(&self) -> &'static str {
		"▌ "
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_tblock_kind_glyphs_and_styles() -> Result<()> {
		assert_eq!(TBlockKind::Prompt.bar_glyph(), "▌ ");
		assert_eq!(TBlockKind::Answer.bar_glyph(), "▌ ");
		assert_eq!(TBlockKind::Error.bar_glyph(), "▌ ");
		assert_eq!(TBlockKind::Running.bar_glyph(), "▌ ");

		assert_eq!(TBlockKind::Prompt.bar_style(), STL_BAR_PROMPT);
		assert_eq!(TBlockKind::Answer.bar_style(), STL_BAR_ANSWER);
		assert_eq!(TBlockKind::Error.bar_style(), STL_BAR_ERR);
		assert_eq!(TBlockKind::Running.bar_style(), STL_BAR_RUNNING);

		Ok(())
	}
}

// endregion: --- Tests
