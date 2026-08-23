// region:    --- Modules

use crate::model::{AirEndState, AirForCreate, AirForUpdate, EpochUs, Id};
use genai::chat::{ChatRequest, ChatResponse, Usage};

// endregion: --- Modules

// region:    --- Types

struct ExtractedUsage {
	token_in: Option<i64>,
	token_out: Option<i64>,
	token_reason: Option<i64>,
	token_cache_hit: Option<i64>,
	token_cache_write: Option<i64>,
}

// endregion: --- Modules

// region:    --- Public Functions

/// Prepares an `AirForCreate` struct with request payloads and initial timestamps.
pub fn prep_air_for_create(
	run_id: Id,
	model_ov: Option<&str>,
	chat_req: &ChatRequest,
	start: EpochUs,
	label: Option<&str>,
) -> AirForCreate {
	let prompt_json = serde_json::to_string(chat_req).ok();

	AirForCreate {
		run_id,
		label: label.map(String::from),
		model_ov: model_ov.map(String::from),
		model_upstream: None,
		prompt_json,
		answer_json: None,
		usage_json: None,
		token_in: None,
		token_out: None,
		token_reason: None,
		token_cache_hit: None,
		token_cache_write: None,
		cost: None,
		error: None,
		end_state: None,
		start: Some(start),
		ai_start: None,
		ai_end: None,
		end: None,
	}
}

/// Prepares an `AirForUpdate` struct from a successful `ChatResponse` with usage metrics.
pub fn prep_air_for_success(
	res: &ChatResponse,
	ai_start: Option<EpochUs>,
	ai_end: Option<EpochUs>,
	end: Option<EpochUs>,
) -> AirForUpdate {
	let model_upstream = Some(res.provider_model_iden.model_name.to_string());
	let answer_json = serde_json::to_string(&res.content).ok();
	let usage_json = serde_json::to_string(&res.usage).ok();

	let ExtractedUsage {
		token_in,
		token_out,
		token_reason,
		token_cache_hit,
		token_cache_write,
	} = extract_usage_metrics(&res.usage);

	AirForUpdate {
		model_upstream,
		answer_json,
		usage_json,
		token_in,
		token_out,
		token_reason,
		token_cache_hit,
		token_cache_write,
		end_state: Some(AirEndState::Success.to_string()),
		ai_start,
		ai_end,
		end,
		..Default::default()
	}
}

/// Prepares an `AirForUpdate` struct for an execution error.
pub fn prep_air_for_error(err_msg: impl Into<String>, end: EpochUs) -> AirForUpdate {
	AirForUpdate {
		error: Some(err_msg.into()),
		end_state: Some(AirEndState::Error.to_string()),
		end: Some(end),
		..Default::default()
	}
}

// endregion: --- Public Functions

// region:    --- Support

/// Extracts token metrics from a `genai::chat::Usage` object.
fn extract_usage_metrics(usage: &Usage) -> ExtractedUsage {
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

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use genai::adapter::AdapterKind;
	use genai::ModelIden;
	use genai::chat::{
		ChatMessage, ChatResponse, CompletionTokensDetails, MessageContent, PromptTokensDetails,
		Usage,
	};
	use uuid::Uuid;

	#[test]
	fn test_prep_air_for_create() -> Result<()> {
		let run_id = Id::from(Uuid::new_v4());
		let req = ChatRequest::from_messages(vec![ChatMessage::user("hello")]);
		let now = EpochUs::now();

		let air_c = prep_air_for_create(run_id, Some("model-a"), &req, now, Some("step-1"));

		assert_eq!(air_c.run_id, run_id);
		assert_eq!(air_c.model_ov.as_deref(), Some("model-a"));
		assert_eq!(air_c.label.as_deref(), Some("step-1"));
		assert_eq!(air_c.start, Some(now));
		let prompt_json = air_c.prompt_json.ok_or("should have prompt_json")?;
		assert!(prompt_json.contains("hello"));

		Ok(())
	}

	#[test]
	fn test_prep_air_for_success() -> Result<()> {
		let usage = Usage {
			prompt_tokens: Some(10),
			completion_tokens: Some(20),
			total_tokens: Some(30),
			prompt_tokens_details: Some(PromptTokensDetails {
				cached_tokens: Some(5),
				audio_tokens: None,
				cache_creation_tokens: None,
				cache_creation_details: None,
			}),
			completion_tokens_details: Some(CompletionTokensDetails {
				reasoning_tokens: Some(8),
				accepted_prediction_tokens: None,
				rejected_prediction_tokens: None,
				audio_tokens: None,
			}),
		};

		let model_iden = ModelIden::from((AdapterKind::OpenAI, "gpt-4o"));

		let res = ChatResponse {
			content: MessageContent::from("ai response text"),
			reasoning_content: None,
			usage,
			model_iden: model_iden.clone(),
			provider_model_iden: model_iden,
			stop_reason: None,
			captured_raw_body: None,
			response_id: None,
		};

		let ai_start = EpochUs::now();
		let ai_end = EpochUs::now();
		let end = EpochUs::now();
		let update = prep_air_for_success(&res, Some(ai_start), Some(ai_end), Some(end));

		assert_eq!(update.model_upstream.as_deref(), Some("gpt-4o"));
		assert_eq!(update.token_in, Some(10));
		assert_eq!(update.token_out, Some(20));
		assert_eq!(update.token_reason, Some(8));
		assert_eq!(update.token_cache_hit, Some(5));
		assert_eq!(update.end_state.as_deref(), Some("success"));
		assert_eq!(update.ai_start, Some(ai_start));
		assert_eq!(update.ai_end, Some(ai_end));
		assert_eq!(update.end, Some(end));

		let answer_json = update.answer_json.ok_or("should have answer_json")?;
		assert!(answer_json.contains("ai response text"));

		Ok(())
	}

	#[test]
	fn test_prep_air_for_error() -> Result<()> {
		let end = EpochUs::now();
		let update = prep_air_for_error("failed to connect", end);

		assert_eq!(update.error.as_deref(), Some("failed to connect"));
		assert_eq!(update.end_state.as_deref(), Some("error"));
		assert_eq!(update.end, Some(end));

		Ok(())
	}
}

// endregion: --- Tests
