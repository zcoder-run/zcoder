use crate::view::style::{
	STL_ANSWER, STL_ANSWER_ERR, STL_ANSWER_ERR_BODY, STL_ANSWER_ERR_CODE, STL_ANSWER_ERR_HDR,
	STL_TBLOCK_RUNNING_ID, STL_TBLOCK_RUNNING_MODEL, STL_TBLOCK_RUNNING_MSG,
};
use crate::view::tblock::TBlockKind;
use ratatui::text::{Line, Span};

/// Creates a styled indicator bar span for the given `TBlockKind`.
pub fn bar_span(kind: TBlockKind) -> Span<'static> {
	Span::styled(kind.bar_glyph(), kind.bar_style())
}

/// Pre-wraps content to fit within `content_width` accounting for the indicator bar width,
/// and prepends `bar_span(kind)` to every resulting visual line.
pub fn build_tblock(kind: TBlockKind, text: &str, content_width: u16) -> Vec<Line<'static>> {
	let bar_width = 2;
	let wrap_width = (content_width as usize).saturating_sub(bar_width).max(1);
	let style = kind.content_style();

	if text.is_empty() {
		return vec![Line::from(vec![
			bar_span(kind),
			Span::styled("", style),
		])];
	}

	let mut lines = Vec::new();
	for raw_line in text.lines() {
		let normalized = raw_line.replace('\t', "    ");
		if normalized.is_empty() {
			lines.push(Line::from(vec![
				bar_span(kind),
				Span::styled("", style),
			]));
		} else {
			let wrapped = textwrap::wrap(&normalized, wrap_width);
			if wrapped.is_empty() {
				lines.push(Line::from(vec![
					bar_span(kind),
					Span::styled("", style),
				]));
			} else {
				for cow_str in wrapped {
					lines.push(Line::from(vec![
						bar_span(kind),
						Span::styled(cow_str.into_owned(), style),
					]));
				}
			}
		}
	}

	if lines.is_empty() {
		lines.push(Line::from(vec![
			bar_span(kind),
			Span::styled("", style),
		]));
	}

	lines
}

/// Builds lines for a prompt block pre-wrapped to `content_width` with prompt indicator bars.
#[allow(dead_code)]
pub fn build_prompt_block(prompt: &str, content_width: u16) -> Vec<Line<'static>> {
	build_tblock(TBlockKind::Prompt, prompt, content_width)
}

/// Builds lines for an answer block pre-wrapped to `content_width` with answer indicator bars.
pub fn build_answer_block(answer: &str, content_width: u16) -> Vec<Line<'static>> {
	build_tblock(TBlockKind::Answer, answer, content_width)
}

/// Builds lines for a single-line running execution block with indicator bar and metadata.
#[allow(dead_code)]
pub fn build_running_block(
	status: &str,
	run_id: Option<&str>,
	model: Option<&str>,
) -> Vec<Line<'static>> {
	let mut spans = vec![
		bar_span(TBlockKind::Running),
		Span::styled(status.to_string(), STL_TBLOCK_RUNNING_MSG),
	];

	if let Some(id) = run_id
		&& !id.is_empty()
	{
		spans.push(Span::styled(format!(" [#{id}]"), STL_TBLOCK_RUNNING_ID));
	}

	if let Some(mdl) = model
		&& !mdl.is_empty()
	{
		spans.push(Span::styled(format!(" [{mdl}]"), STL_TBLOCK_RUNNING_MODEL));
	}

	vec![Line::from(spans)]
}

/// Builds lines for an error block pre-wrapped to `content_width` with error indicator bars, header formatting, and stack trace styles.
pub fn build_error_block(err: &str, content_width: u16) -> Vec<Line<'static>> {
	let bar_width = 2;
	let wrap_width = (content_width as usize).saturating_sub(bar_width).max(1);
	let mut lines = Vec::new();
	let mut in_stack_trace = false;

	for (idx, raw_line) in err.lines().enumerate() {
		let line = raw_line.replace('\t', "    ");
		let trimmed = line.trim();

		if idx == 0 {
			let prefix = if trimmed.starts_with("Error:")
				|| trimmed.starts_with("Lua Execution Error")
				|| trimmed.starts_with("Lua Error")
			{
				""
			} else {
				"Error: "
			};
			let full_hdr = format!("{prefix}{line}");
			let wrapped = textwrap::wrap(&full_hdr, wrap_width.saturating_sub(2).max(1));
			if wrapped.is_empty() {
				lines.push(Line::from(vec![
					bar_span(TBlockKind::Error),
					Span::styled("✘ ", STL_ANSWER_ERR),
					Span::styled(full_hdr, STL_ANSWER_ERR_HDR),
				]));
			} else {
				for (w_idx, w_line) in wrapped.into_iter().enumerate() {
					if w_idx == 0 {
						lines.push(Line::from(vec![
							bar_span(TBlockKind::Error),
							Span::styled("✘ ", STL_ANSWER_ERR),
							Span::styled(w_line.into_owned(), STL_ANSWER_ERR_HDR),
						]));
					} else {
						lines.push(Line::from(vec![
							bar_span(TBlockKind::Error),
							Span::styled("  ", STL_ANSWER_ERR),
							Span::styled(w_line.into_owned(), STL_ANSWER_ERR_HDR),
						]));
					}
				}
			}
		} else if trimmed.starts_with("Stack traceback:")
			|| trimmed.starts_with("stack traceback:")
			|| trimmed.starts_with("Stack Trace:")
			|| trimmed.starts_with("stack trace:")
		{
			in_stack_trace = true;
			let wrapped = textwrap::wrap(&line, wrap_width);
			if wrapped.is_empty() {
				lines.push(Line::from(vec![
					bar_span(TBlockKind::Error),
					Span::styled(line, STL_ANSWER_ERR_HDR),
				]));
			} else {
				for w_line in wrapped {
					lines.push(Line::from(vec![
						bar_span(TBlockKind::Error),
						Span::styled(w_line.into_owned(), STL_ANSWER_ERR_HDR),
					]));
				}
			}
		} else if in_stack_trace
			|| trimmed.starts_with("```")
			|| trimmed.starts_with("-->")
			|| trimmed.contains(" | ")
			|| (trimmed.ends_with('|') && trimmed.chars().all(|c| c.is_ascii_digit() || c.is_whitespace() || c == '|'))
		{
			let wrapped = textwrap::wrap(&line, wrap_width);
			if wrapped.is_empty() {
				lines.push(Line::from(vec![
					bar_span(TBlockKind::Error),
					Span::styled(line, STL_ANSWER_ERR_CODE),
				]));
			} else {
				for w_line in wrapped {
					lines.push(Line::from(vec![
						bar_span(TBlockKind::Error),
						Span::styled(w_line.into_owned(), STL_ANSWER_ERR_CODE),
					]));
				}
			}
		} else if trimmed.is_empty() {
			lines.push(Line::from(vec![
				bar_span(TBlockKind::Error),
				Span::styled("", STL_ANSWER),
			]));
		} else {
			let wrapped = textwrap::wrap(&line, wrap_width);
			if wrapped.is_empty() {
				lines.push(Line::from(vec![
					bar_span(TBlockKind::Error),
					Span::styled(line, STL_ANSWER_ERR_BODY),
				]));
			} else {
				for w_line in wrapped {
					lines.push(Line::from(vec![
						bar_span(TBlockKind::Error),
						Span::styled(w_line.into_owned(), STL_ANSWER_ERR_BODY),
					]));
				}
			}
		}
	}

	if lines.is_empty() {
		lines.push(Line::from(vec![
			bar_span(TBlockKind::Error),
			Span::styled("✘ ", STL_ANSWER_ERR),
			Span::styled("Error: Unknown error", STL_ANSWER_ERR_HDR),
		]));
	}

	lines
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_tblock_builder_prompt() -> Result<()> {
		// -- Setup & Fixtures
		let prompt = "Explain quantum computing\nIn simple terms";

		// -- Exec
		let lines = build_prompt_block(prompt, 80);

		// -- Check
		assert_eq!(lines.len(), 2);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[0].style, TBlockKind::Prompt.bar_style());
		assert_eq!(lines[0].spans[1].content, "Explain quantum computing");
		assert_eq!(lines[1].spans[0].content, "▌ ");
		assert_eq!(lines[1].spans[1].content, "In simple terms");

		Ok(())
	}

	#[test]
	fn test_tblock_builder_answer() -> Result<()> {
		// -- Setup & Fixtures
		let answer = "Quantum computing uses qubits.\nSuperposition allows parallelism.";

		// -- Exec
		let lines = build_answer_block(answer, 80);

		// -- Check
		assert_eq!(lines.len(), 2);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[0].style, TBlockKind::Answer.bar_style());
		assert_eq!(lines[0].spans[1].content, "Quantum computing uses qubits.");
		assert_eq!(lines[1].spans[0].content, "▌ ");
		assert_eq!(lines[1].spans[1].content, "Superposition allows parallelism.");

		Ok(())
	}

	#[test]
	fn test_tblock_builder_wrapping() -> Result<()> {
		// -- Setup & Fixtures
		let text = "One two three four five six seven eight nine ten";

		// -- Exec
		// content_width = 16 -> wrap_width = 16 - 2 = 14 chars
		let lines = build_tblock(TBlockKind::Answer, text, 16);

		// -- Check
		assert!(lines.len() > 1);
		for line in &lines {
			assert_eq!(line.spans[0].content, "▌ ");
			assert_eq!(line.spans[0].style, TBlockKind::Answer.bar_style());
		}

		Ok(())
	}

	#[test]
	fn test_tblock_builder_running() -> Result<()> {
		// -- Setup & Fixtures
		let status = "Generating response";
		let run_id = Some("run-42");
		let model = Some("gpt-4o");

		// -- Exec
		let lines = build_running_block(status, run_id, model);

		// -- Check
		assert_eq!(lines.len(), 1);
		let line = &lines[0];
		assert_eq!(line.spans.len(), 4);
		assert_eq!(line.spans[0].content, "▌ ");
		assert_eq!(line.spans[0].style, TBlockKind::Running.bar_style());
		assert_eq!(line.spans[1].content, "Generating response");
		assert_eq!(line.spans[2].content, " [#run-42]");
		assert_eq!(line.spans[3].content, " [gpt-4o]");

		Ok(())
	}

	#[test]
	fn test_tblock_builder_running_minimal() -> Result<()> {
		// -- Setup & Fixtures
		let status = "Executing";

		// -- Exec
		let lines = build_running_block(status, None, None);

		// -- Check
		assert_eq!(lines.len(), 1);
		let line = &lines[0];
		assert_eq!(line.spans.len(), 2);
		assert_eq!(line.spans[0].content, "▌ ");
		assert_eq!(line.spans[0].style, TBlockKind::Running.bar_style());
		assert_eq!(line.spans[1].content, "Executing");

		Ok(())
	}

	#[test]
	fn test_tblock_builder_error_simple() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Network connection failed";

		// -- Exec
		let lines = build_error_block(err, 80);

		// -- Check
		assert_eq!(lines.len(), 1);
		let line = &lines[0];
		assert_eq!(line.spans.len(), 3);
		assert_eq!(line.spans[0].content, "▌ ");
		assert_eq!(line.spans[0].style, TBlockKind::Error.bar_style());
		assert_eq!(line.spans[1].content, "✘ ");
		assert_eq!(line.spans[2].content, "Error: Network connection failed");

		Ok(())
	}

	#[test]
	fn test_tblock_builder_error_stack_trace() -> Result<()> {
		// -- Setup & Fixtures
		let err = "Lua Error (line 12): division by zero\n```lua\n12 | local val = 1 / 0\n```\nStack Trace:\n\t[C]: in function 'error'\n\tscript.lua:12: in main chunk";

		// -- Exec
		let lines = build_error_block(err, 80);

		// -- Check
		assert_eq!(lines.len(), 7);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Lua Error (line 12): division by zero");
		assert_eq!(lines[1].spans[0].content, "▌ ");
		assert_eq!(lines[1].spans[1].content, "```lua");
		assert_eq!(lines[4].spans[0].content, "▌ ");
		assert_eq!(lines[4].spans[1].content, "Stack Trace:");
		assert_eq!(lines[5].spans[0].content, "▌ ");
		assert_eq!(lines[5].spans[1].content, "    [C]: in function 'error'");

		Ok(())
	}

	#[test]
	fn test_tblock_builder_error_empty() -> Result<()> {
		// -- Setup & Fixtures
		let err = "";

		// -- Exec
		let lines = build_error_block(err, 80);

		// -- Check
		assert_eq!(lines.len(), 1);
		assert_eq!(lines[0].spans[0].content, "▌ ");
		assert_eq!(lines[0].spans[1].content, "✘ ");
		assert_eq!(lines[0].spans[2].content, "Error: Unknown error");

		Ok(())
	}
}

// endregion: --- Tests
