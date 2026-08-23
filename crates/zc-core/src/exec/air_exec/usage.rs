// region:    --- Modules

use genai::chat::Usage;

// endregion: --- Modules

// region:    --- Types

#[derive(Debug, Clone, Default)]
pub struct ExtractedUsage {
	pub token_in: Option<i64>,
	pub token_out: Option<i64>,
	pub token_reason: Option<i64>,
	pub token_cache_hit: Option<i64>,
	pub token_cache_write: Option<i64>,
}

// endregion: --- Types

// region:    --- Public Functions

/// Extracts token metrics from a `genai::chat::Usage` object.
pub fn extract_usage_metrics(usage: &Usage) -> ExtractedUsage {
	let token_in = usage.prompt_tokens.map(i64::from);
	let token_out = usage.completion_tokens.map(i64::from);

	let token_reason = usage
		.completion_tokens_details
		.as_ref()
		.and_then(|d| d.reasoning_tokens)
		.map(i64::from);

	let token_cache_hit = usage
		.prompt_tokens_details
		.as_ref()
		.and_then(|d| d.cached_tokens)
		.map(i64::from);

	let token_cache_write = None;

	ExtractedUsage {
		token_in,
		token_out,
		token_reason,
		token_cache_hit,
		token_cache_write,
	}
}

// endregion: --- Public Functions

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use genai::chat::{CompletionTokensDetails, PromptTokensDetails, Usage};

	#[test]
	fn test_air_exec_extract_usage_metrics_full() -> Result<()> {
		// -- Setup & Fixtures
		let usage = Usage {
			prompt_tokens: Some(100),
			completion_tokens: Some(50),
			total_tokens: Some(150),
			prompt_tokens_details: Some(PromptTokensDetails {
				cached_tokens: Some(25),
				audio_tokens: None,
				cache_creation_tokens: None,
				cache_creation_details: None,
			}),
			completion_tokens_details: Some(CompletionTokensDetails {
				reasoning_tokens: Some(15),
				accepted_prediction_tokens: None,
				rejected_prediction_tokens: None,
				audio_tokens: None,
			}),
		};

		// -- Exec
		let metrics = extract_usage_metrics(&usage);

		// -- Check
		assert_eq!(metrics.token_in, Some(100));
		assert_eq!(metrics.token_out, Some(50));
		assert_eq!(metrics.token_reason, Some(15));
		assert_eq!(metrics.token_cache_hit, Some(25));
		assert_eq!(metrics.token_cache_write, None);

		Ok(())
	}

	#[test]
	fn test_air_exec_extract_usage_metrics_empty() -> Result<()> {
		// -- Setup & Fixtures
		let usage = Usage {
			prompt_tokens: None,
			completion_tokens: None,
			total_tokens: None,
			prompt_tokens_details: None,
			completion_tokens_details: None,
		};

		// -- Exec
		let metrics = extract_usage_metrics(&usage);

		// -- Check
		assert_eq!(metrics.token_in, None);
		assert_eq!(metrics.token_out, None);
		assert_eq!(metrics.token_reason, None);
		assert_eq!(metrics.token_cache_hit, None);
		assert_eq!(metrics.token_cache_write, None);

		Ok(())
	}
}

// endregion: --- Tests
