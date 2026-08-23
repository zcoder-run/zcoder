use crate::view::style::{
	STL_BAR_ANSWER, STL_BAR_ERR, STL_BAR_PROMPT, STL_BAR_WORK, STL_TBLOCK_ANSWER, STL_TBLOCK_PROMPT,
	STL_TBLOCK_WORK_MSG,
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
	Work,
	Running,
}

/// Information used to render an AI work block in running or completed state.
#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiWorkInfo {
	pub is_running: bool,
	pub model: Option<String>,
	pub duration: Option<String>,
	pub input_tokens: Option<u32>,
	pub output_tokens: Option<u32>,
	pub reasoning_tokens: Option<u32>,
}

// endregion: --- Types

impl TBlockKind {
	/// Returns the indicator bar style for this block kind.
	pub fn bar_style(&self) -> Style {
		match self {
			Self::Prompt => STL_BAR_PROMPT,
			Self::Answer => STL_BAR_ANSWER,
			Self::Error => STL_BAR_ERR,
			Self::Work | Self::Running => STL_BAR_WORK,
		}
	}

	/// Returns the default body text style for this block kind.
	#[allow(dead_code)]
	pub fn content_style(&self) -> Style {
		match self {
			Self::Prompt => STL_TBLOCK_PROMPT,
			Self::Answer => STL_TBLOCK_ANSWER,
			Self::Error => STL_BAR_ERR,
			Self::Work | Self::Running => STL_TBLOCK_WORK_MSG,
		}
	}

	/// Returns the vertical bar glyph string used for the indicator bar.
	pub fn bar_glyph(&self) -> &'static str {
		"▌ "
	}
}

#[allow(dead_code)]
impl AiWorkInfo {
	pub fn new(is_running: bool) -> Self {
		Self {
			is_running,
			..Default::default()
		}
	}

	pub fn with_model(mut self, model: impl Into<String>) -> Self {
		self.model = Some(model.into());
		self
	}

	pub fn with_duration(mut self, duration: impl Into<String>) -> Self {
		self.duration = Some(duration.into());
		self
	}

	pub fn with_tokens(
		mut self,
		input: Option<u32>,
		output: Option<u32>,
		reasoning: Option<u32>,
	) -> Self {
		self.input_tokens = input;
		self.output_tokens = output;
		self.reasoning_tokens = reasoning;
		self
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
		assert_eq!(TBlockKind::Work.bar_glyph(), "▌ ");
		assert_eq!(TBlockKind::Running.bar_glyph(), "▌ ");

		assert_eq!(TBlockKind::Prompt.bar_style(), STL_BAR_PROMPT);
		assert_eq!(TBlockKind::Answer.bar_style(), STL_BAR_ANSWER);
		assert_eq!(TBlockKind::Error.bar_style(), STL_BAR_ERR);
		assert_eq!(TBlockKind::Work.bar_style(), STL_BAR_WORK);
		assert_eq!(TBlockKind::Running.bar_style(), STL_BAR_WORK);

		Ok(())
	}
}

// endregion: --- Tests
