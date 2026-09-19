use crate::exec::Result;
use crate::exec::air_exec::pricing::price_it;
use crate::exec::air_exec::usage::{ExtractedUsage, extract_usage_metrics};
use crate::model::{AirBmc, AirEndState, AirForCreate, AirForUpdate, EpochUs, Id, ModelManager, RunBmc};
use genai::chat::{ChatOptions, ChatRequest, ChatResponse};

/// Executes an AI chat request, automatically recording creation, timing, and update on the Air model entity.
pub async fn exec_air_chat(
	mm: &'static ModelManager,
	client: &genai::Client,
	model: &str,
	chat_req: ChatRequest,
	run_id: Id,
	wspace_id: Option<Id>,
	label: Option<&str>,
) -> Result<(ChatResponse, Id)> {
	let start = EpochUs::now();
	let air_c = prep_air_for_create(run_id, wspace_id, Some(model), &chat_req, start, label);
	let air_id = AirBmc::create_next(mm, run_id, air_c).await?;

	let ai_start = EpochUs::now();

	// For development, we capture the raw body
	let options = ChatOptions::default().with_capture_raw_body(true);

	let chat_res = match client.exec_chat(model, chat_req, Some(&options)).await {
		Ok(res) => {
			let ai_end = EpochUs::now();
			let end = ai_end;
			let air_u = prep_air_for_success(&res, Some(ai_start), Some(ai_end), Some(end));
			let _ = AirBmc::update(mm, air_id, air_u).await;
			let _ = RunBmc::recompute_total_cost(mm, run_id).await;
			res
		}
		Err(err) => {
			let ai_end = EpochUs::now();
			let air_u = prep_air_for_error(err.to_string(), ai_end);
			let _ = AirBmc::update(mm, air_id, air_u).await;
			return Err(err.into());
		}
	};

	Ok((chat_res, air_id))
}

/// Prepares an `AirForCreate` struct with request payloads and initial timestamps.
pub fn prep_air_for_create(
	run_id: Id,
	wspace_id: Option<Id>,
	model_ov: Option<&str>,
	chat_req: &ChatRequest,
	start: EpochUs,
	label: Option<&str>,
) -> AirForCreate {
	let prompt_json = serde_json::to_string(chat_req).ok();

	AirForCreate {
		run_id,
		wspace_id,
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

	let cost = price_it(
		res.provider_model_iden.adapter_kind.as_lower_str(),
		&res.provider_model_iden.model_name,
		&res.usage,
	)
	.map(|p| p.cost);

	AirForUpdate {
		model_upstream,
		answer_json,
		usage_json,
		token_in,
		token_out,
		token_reason,
		token_cache_hit,
		token_cache_write,
		cost,
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

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::model::{RunForCreate, get_model_manager};
	use genai::ModelIden;
	use genai::adapter::AdapterKind;
	use genai::chat::{ChatMessage, ChatResponse, CompletionTokensDetails, MessageContent, PromptTokensDetails, Usage};
	use uuid::Uuid;

	#[test]
	fn test_air_exec_prep_air_for_create() -> Result<()> {
		// -- Setup & Fixtures
		let run_id = Id::from(Uuid::new_v4());
		let req = ChatRequest::from_messages(vec![ChatMessage::user("hello")]);
		let now = EpochUs::now();

		// -- Exec
		let air_c = prep_air_for_create(run_id, None, Some("model-a"), &req, now, Some("step-1"));

		// -- Check
		assert_eq!(air_c.run_id, run_id);
		assert_eq!(air_c.model_ov.as_deref(), Some("model-a"));
		assert_eq!(air_c.label.as_deref(), Some("step-1"));
		assert_eq!(air_c.start, Some(now));
		let prompt_json = air_c.prompt_json.ok_or("should have prompt_json")?;
		assert!(prompt_json.contains("hello"));

		Ok(())
	}

	#[test]
	fn test_air_exec_prep_air_for_success() -> Result<()> {
		// -- Setup & Fixtures
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

		// -- Exec
		let update = prep_air_for_success(&res, Some(ai_start), Some(ai_end), Some(end));

		// -- Check
		assert_eq!(update.model_upstream.as_deref(), Some("gpt-4o"));
		assert_eq!(update.token_in, Some(10));
		assert_eq!(update.token_out, Some(20));
		assert_eq!(update.token_reason, Some(8));
		assert_eq!(update.token_cache_hit, Some(5));
		assert!(update.cost.is_some());
		assert!(update.cost.ok_or("should have cost")? > 0.0);
		assert_eq!(update.end_state.as_deref(), Some("success"));
		assert_eq!(update.ai_start, Some(ai_start));
		assert_eq!(update.ai_end, Some(ai_end));
		assert_eq!(update.end, Some(end));

		let answer_json = update.answer_json.ok_or("should have answer_json")?;
		assert!(answer_json.contains("ai response text"));

		Ok(())
	}

	#[test]
	fn test_air_exec_prep_air_for_error() -> Result<()> {
		// -- Setup & Fixtures
		let end = EpochUs::now();

		// -- Exec
		let update = prep_air_for_error("failed to connect", end);

		// -- Check
		assert_eq!(update.error.as_deref(), Some("failed to connect"));
		assert_eq!(update.end_state.as_deref(), Some("error"));
		assert_eq!(update.end, Some(end));

		Ok(())
	}

	#[tokio::test]
	async fn test_model_air_bmc_full_lifecycle_and_metrics() -> Result<()> {
		// -- Setup & Fixtures
		let mm = get_model_manager()?;
		let run_c = RunForCreate {
			wspace_id: None,
			prompt: Some("full lifecycle test".to_string()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c).await?;

		// -- Exec: Prep & Create Air
		let start = EpochUs::now();
		let chat_req = genai::chat::ChatRequest::from_messages(vec![genai::chat::ChatMessage::user("count to three")]);
		let air_c = prep_air_for_create(run_id, None, Some("test-model"), &chat_req, start, Some("step-label"));
		let air_id = AirBmc::create_next(mm, run_id, air_c).await?;

		// -- Check: Initial Air State
		let air = AirBmc::get(mm, air_id).await?;
		assert_eq!(air.run_id, run_id);
		assert_eq!(air.idx, 1);
		assert_eq!(air.model_ov.as_deref(), Some("test-model"));
		assert_eq!(air.label.as_deref(), Some("step-label"));
		assert_eq!(air.start, Some(start));
		assert!(air.prompt_json.ok_or("should have prompt_json")?.contains("count to three"));

		// -- Exec: Update with Success
		let model_iden = genai::ModelIden::from((genai::adapter::AdapterKind::OpenAI, "gpt-4o-mini"));
		let usage = genai::chat::Usage {
			prompt_tokens: Some(15),
			completion_tokens: Some(25),
			total_tokens: Some(40),
			prompt_tokens_details: Some(genai::chat::PromptTokensDetails {
				cached_tokens: Some(5),
				audio_tokens: None,
				cache_creation_tokens: None,
				cache_creation_details: None,
			}),
			completion_tokens_details: Some(genai::chat::CompletionTokensDetails {
				reasoning_tokens: Some(7),
				accepted_prediction_tokens: None,
				rejected_prediction_tokens: None,
				audio_tokens: None,
			}),
		};
		let res = genai::chat::ChatResponse {
			content: genai::chat::MessageContent::from("one two three"),
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
		AirBmc::update(mm, air_id, update).await?;

		// -- Check: Updated Air State
		let air = AirBmc::get(mm, air_id).await?;
		assert_eq!(air.model_upstream.as_deref(), Some("gpt-4o-mini"));
		assert_eq!(air.token_in, Some(15));
		assert_eq!(air.token_out, Some(25));
		assert_eq!(air.token_reason, Some(7));
		assert_eq!(air.token_cache_hit, Some(5));
		assert_eq!(air.end_state.as_deref(), Some("success"));
		assert_eq!(air.ai_start, Some(ai_start));
		assert_eq!(air.ai_end, Some(ai_end));
		assert_eq!(air.end, Some(end));
		assert!(air.answer_json.ok_or("should have answer_json")?.contains("one two three"));

		// -- Exec & Check: Error Case
		let air_err_c = air_for_create(run_id);
		let err_air_id = AirBmc::create_next(mm, run_id, air_err_c).await?;
		let err_end = EpochUs::now();
		let err_update = prep_air_for_error("connection refused", err_end);
		AirBmc::update(mm, err_air_id, err_update).await?;

		let err_air = AirBmc::get(mm, err_air_id).await?;
		assert_eq!(err_air.idx, 2);
		assert_eq!(err_air.error.as_deref(), Some("connection refused"));
		assert_eq!(err_air.end_state.as_deref(), Some("error"));
		assert_eq!(err_air.end, Some(err_end));

		Ok(())
	}

	// -- Test Support

	fn air_for_create(run_id: Id) -> AirForCreate {
		AirForCreate {
			run_id,
			wspace_id: None,
			label: None,
			model_ov: None,
			model_upstream: None,
			prompt_json: None,
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
			start: None,
			ai_start: None,
			ai_end: None,
			end: None,
		}
	}
}

// endregion: --- Tests
