#![allow(unused)]
use super::TuiState;
use super::tui_state_base::format_duration_us;
use crate::view::tblock::AiWorkInfo;
use zc_router::{RouterClient, db_size};

const SYS_REFRESH_INTERVAL_US: i64 = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptSubmit {
	Ignored,

	/// A `\` before the cursor was replaced by a newline, and nothing is sent.
	Newline,
	Quit,
	Run(String),
}

pub struct StateProcessor;

impl StateProcessor {
	pub fn submit_prompt(state: &mut TuiState) -> PromptSubmit {
		if state.prompter_mut().continue_line() {
			return PromptSubmit::Newline;
		}
		let trimmed = state.prompter().text().trim().to_string();
		if trimmed == "/q" {
			return PromptSubmit::Quit;
		}
		if trimmed.is_empty() || state.is_waiting() {
			return PromptSubmit::Ignored;
		}
		state.prompter_mut().take_text();
		state.set_last_prompt(Some(trimmed.clone()));
		state.set_waiting(true);
		state.set_last_error(None);
		PromptSubmit::Run(trimmed)
	}

	pub fn apply_prompt_send_error(state: &mut TuiState, prompt: String, error: String) {
		Self::apply_run_error(state, format!("Failed to send prompt: {error}"));
		state.prompter_mut().restore_text(&prompt);
	}

	pub fn copy_prompt(state: &mut TuiState) {
		let text = state.prompter().selected_text().unwrap_or(state.prompter().text()).to_string();
		if text.is_empty() {
			state.set_status("Nothing to copy".to_string());
			return;
		}
		let status = match state.clipboard_mut().set_text(text) {
			Ok(()) => "Copied to clipboard".to_string(),
			Err(err) => format!("Clipboard error: {err}"),
		};
		state.set_status(status);
	}

	pub fn paste_from_clipboard(state: &mut TuiState) {
		match state.clipboard_mut().get_text() {
			Ok(text) => state.prompter_mut().insert_str(&text),
			Err(err) => state.set_status(format!("Clipboard error: {err}")),
		}
	}

	pub fn apply_run_start(state: &mut TuiState) {
		state.set_status("Sending to AI...".to_string());
		let now_us = zc_common::time::now_micro();
		state.set_work_start_us(Some(now_us));
		state.set_ai_work_info(Some(AiWorkInfo::new(true).with_duration("0ms")));
	}

	pub fn apply_ai_start(state: &mut TuiState, model: Option<String>, start_us: i64) {
		state.set_work_start_us(Some(start_us));
		let mut info = AiWorkInfo::new(true).with_duration("0ms");
		if let Some(m) = model {
			info = info.with_model(m);
		}
		state.set_ai_work_info(Some(info));
	}

	pub fn apply_ai_done(
		state: &mut TuiState,
		model: Option<String>,
		duration_us: Option<i64>,
		cost: Option<f64>,
		tokens: (Option<u32>, Option<u32>, Option<u32>),
	) {
		let mut info = AiWorkInfo::new(false).with_cost(cost).with_tokens(tokens.0, tokens.1, tokens.2);
		if let Some(m) = model {
			info = info.with_model(m);
		}
		if let Some(us) = duration_us {
			info = info.with_duration(format_duration_us(us));
		} else if let Some(start_us) = state.work_start_us() {
			let elapsed = (zc_common::time::now_micro() - start_us).max(0);
			info = info.with_duration(format_duration_us(elapsed));
		}
		state.set_ai_work_info(Some(info));
	}

	pub fn apply_run_end(state: &mut TuiState) {
		state.set_waiting(false);
		state.set_status("Idle".to_string());
		if let Some(info) = state.ai_work_info_mut() {
			info.is_running = false;
		}
	}

	pub fn apply_run_result(state: &mut TuiState, answer: String) {
		state.set_last_answer(Some(answer));
	}

	pub fn apply_run_error(state: &mut TuiState, error: String) {
		state.set_waiting(false);
		state.set_status("Error".to_string());
		state.set_last_error(Some(error));
		if let Some(info) = state.ai_work_info_mut() {
			info.is_running = false;
		}
	}

	pub fn apply_tick(state: &mut TuiState, ts: i64) {
		state.update_elapsed_time(ts);
	}

	pub async fn process_sys_metrics(state: &mut TuiState, client: &RouterClient) {
		if !state.show_sys_states() {
			return;
		}

		let now_us = zc_common::time::now_micro();
		if let Some(last_us) = state.sys_refresh_us()
			&& now_us - last_us < SYS_REFRESH_INTERVAL_US
		{
			return;
		}
		state.set_sys_refresh_us(Some(now_us));

		state.refresh_sys_state();
		if let Ok(size) = db_size(client).await {
			state.set_db_memory(size.max(0) as u64);
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::core::_test_support::start_router_with_stub;
	use zc_common::event_base::new_mpsc_bounded;

	#[tokio::test]
	async fn test_state_processor_sys_metrics_when_inactive() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let client = start_router_with_stub()?;

		assert!(!state.show_sys_states());
		assert_eq!(state.memory(), 0);
		assert_eq!(state.db_memory(), 0);

		// -- Exec
		StateProcessor::process_sys_metrics(&mut state, &client).await;

		// -- Check
		assert_eq!(state.memory(), 0);
		assert_eq!(state.db_memory(), 0);

		Ok(())
	}

	#[tokio::test]
	async fn test_state_processor_sys_metrics_when_active() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.set_show_sys_states(true);
		let client = start_router_with_stub()?;

		// -- Exec
		StateProcessor::process_sys_metrics(&mut state, &client).await;

		// -- Check
		assert!(state.memory() > 0);
		assert!(state.db_memory() > 0);

		Ok(())
	}

	#[tokio::test]
	async fn test_state_processor_sys_metrics_throttled() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.set_show_sys_states(true);
		let client = start_router_with_stub()?;

		// -- Exec
		StateProcessor::process_sys_metrics(&mut state, &client).await;
		let first_refresh = state.sys_refresh_us();
		StateProcessor::process_sys_metrics(&mut state, &client).await;

		// -- Check
		assert!(first_refresh.is_some());
		assert_eq!(state.sys_refresh_us(), first_refresh);

		Ok(())
	}

	#[test]
	fn test_state_processor_apply_run_error() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.set_waiting(true);
		state.set_status("Sending to AI...".to_string());

		// -- Exec
		StateProcessor::apply_run_error(&mut state, "Execution failed".to_string());

		// -- Check
		assert!(!state.is_waiting());
		assert_eq!(state.status(), "Error");
		assert_eq!(state.last_error(), Some("Execution failed"));

		Ok(())
	}

	#[test]
	fn test_state_processor_submit_prompt_run() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("  Why is the sky blue?  ".to_string()));

		// -- Exec
		let submit = StateProcessor::submit_prompt(&mut state);

		// -- Check
		assert_eq!(submit, PromptSubmit::Run("Why is the sky blue?".to_string()));
		assert_eq!(state.prompter().text(), "");
		assert_eq!(state.last_prompt(), Some("Why is the sky blue?"));
		assert!(state.is_waiting());

		Ok(())
	}

	#[test]
	fn test_state_processor_submit_prompt_ignored_when_empty() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("   ".to_string()));

		// -- Exec
		let submit = StateProcessor::submit_prompt(&mut state);

		// -- Check
		assert_eq!(submit, PromptSubmit::Ignored);
		assert_eq!(state.prompter().text(), "   ");
		assert!(!state.is_waiting());

		Ok(())
	}

	#[test]
	fn test_state_processor_submit_prompt_quit() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some(" /q ".to_string()));
		state.set_waiting(true);

		// -- Exec
		let submit = StateProcessor::submit_prompt(&mut state);

		// -- Check
		assert_eq!(submit, PromptSubmit::Quit);

		Ok(())
	}

	#[test]
	fn test_state_processor_submit_prompt_ignored_while_waiting() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("next".to_string()));
		state.set_waiting(true);

		// -- Exec
		let submit = StateProcessor::submit_prompt(&mut state);

		// -- Check
		assert_eq!(submit, PromptSubmit::Ignored);
		assert_eq!(state.prompter().text(), "next");

		Ok(())
	}

	#[test]
	fn test_state_processor_submit_prompt_line_continuation() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("first\\".to_string()));
		state.set_waiting(true);

		// -- Exec
		let submit = StateProcessor::submit_prompt(&mut state);

		// -- Check
		assert_eq!(submit, PromptSubmit::Newline);
		assert_eq!(state.prompter().text(), "first\n");

		Ok(())
	}

	#[test]
	fn test_state_processor_submit_prompt_typing_after_submit() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("first".to_string()));
		let submit = StateProcessor::submit_prompt(&mut state);

		// -- Exec
		state.prompter_mut().insert_char('x');

		// -- Check
		assert!(matches!(submit, PromptSubmit::Run(_)));
		assert_eq!(state.prompter().text(), "x");

		Ok(())
	}

	#[test]
	fn test_state_processor_apply_prompt_send_error_restores_empty() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.set_waiting(true);

		// -- Exec
		StateProcessor::apply_prompt_send_error(&mut state, "draft".to_string(), "router down".to_string());

		// -- Check
		assert!(!state.is_waiting());
		assert_eq!(state.last_error(), Some("Failed to send prompt: router down"));
		assert_eq!(state.prompter().text(), "draft");

		Ok(())
	}

	#[test]
	fn test_state_processor_apply_prompt_send_error_prepends_existing() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("new text".to_string()));

		// -- Exec
		StateProcessor::apply_prompt_send_error(&mut state, "draft".to_string(), "router down".to_string());

		// -- Check
		assert_eq!(state.prompter().text(), "draft\nnew text");

		Ok(())
	}

	#[test]
	fn test_state_processor_copy_prompt_empty() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);

		// -- Exec
		StateProcessor::copy_prompt(&mut state);

		// -- Check
		assert_eq!(state.status(), "Nothing to copy");

		Ok(())
	}

	#[test]
	fn test_state_processor_ai_lifecycle() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);

		// -- Exec: AI Start
		StateProcessor::apply_ai_start(&mut state, Some("claude-3-5-sonnet".to_string()), 1_000_000);
		let info = state.ai_work_info().ok_or("should have work info")?;
		assert!(info.is_running);
		assert_eq!(info.model.as_deref(), Some("claude-3-5-sonnet"));
		assert_eq!(info.duration.as_deref(), Some("0ms"));

		// -- Exec: Tick
		state.update_elapsed_time(3_500_000);
		let info = state.ai_work_info().ok_or("should have work info")?;
		assert_eq!(info.duration.as_deref(), Some("2s 500ms"));

		// -- Exec: AI Done
		StateProcessor::apply_ai_done(
			&mut state,
			Some("claude-3-5-sonnet".to_string()),
			Some(4_500_000),
			Some(0.05),
			(Some(120), Some(450), Some(80)),
		);
		let info = state.ai_work_info().ok_or("should have work info")?;
		assert!(!info.is_running);
		assert_eq!(info.duration.as_deref(), Some("4s 500ms"));
		assert_eq!(info.cost, Some(0.05));
		assert_eq!(info.input_tokens, Some(120));
		assert_eq!(info.output_tokens, Some(450));
		assert_eq!(info.reasoning_tokens, Some(80));

		Ok(())
	}
}

// endregion: --- Tests
