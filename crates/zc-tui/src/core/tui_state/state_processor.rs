#![allow(unused)]
use super::TuiState;
use super::tui_state_base::format_duration_us;
use crate::view::tblock::AiWorkInfo;
use zc_core::model::get_model_manager;

pub struct StateProcessor;

impl StateProcessor {
	pub fn start_prompt_run(state: &mut TuiState) {
		let prompt = state.input().to_string();
		if !prompt.is_empty() {
			state.set_last_prompt(Some(prompt));
		}
		state.clear_input();
		state.set_waiting(true);
		state.set_last_error(None);
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
		tokens: (Option<u32>, Option<u32>, Option<u32>),
	) {
		let mut info = AiWorkInfo::new(false).with_tokens(tokens.0, tokens.1, tokens.2);
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

	pub async fn process_sys_metrics(state: &mut TuiState) {
		if !state.show_sys_states() {
			return;
		}

		state.refresh_sys_state();
		if let Ok(mm) = get_model_manager()
			&& let Ok(db_size) = mm.db_size().await
		{
			state.set_db_memory(db_size.max(0) as u64);
		}
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[tokio::test]
	async fn test_state_processor_sys_metrics_when_inactive() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		assert!(!state.show_sys_states());
		assert_eq!(state.memory(), 0);
		assert_eq!(state.db_memory(), 0);

		// -- Exec
		StateProcessor::process_sys_metrics(&mut state).await;

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

		// -- Exec
		StateProcessor::process_sys_metrics(&mut state).await;

		// -- Check
		assert!(state.memory() > 0);
		assert!(state.db_memory() > 0);

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
	fn test_state_processor_start_prompt_run() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(Some("Why is the sky blue?".to_string()));

		// -- Exec
		StateProcessor::start_prompt_run(&mut state);

		// -- Check
		assert_eq!(state.input(), "");
		assert_eq!(state.last_prompt(), Some("Why is the sky blue?"));
		assert!(state.is_waiting());

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
		StateProcessor::apply_ai_done(&mut state, Some("claude-3-5-sonnet".to_string()), Some(4_500_000), (Some(120), Some(450), Some(80)));
		let info = state.ai_work_info().ok_or("should have work info")?;
		assert!(!info.is_running);
		assert_eq!(info.duration.as_deref(), Some("4s 500ms"));
		assert_eq!(info.input_tokens, Some(120));
		assert_eq!(info.output_tokens, Some(450));
		assert_eq!(info.reasoning_tokens, Some(80));

		Ok(())
	}
}

// endregion: --- Tests
