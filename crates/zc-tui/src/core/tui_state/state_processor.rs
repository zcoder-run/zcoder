#![allow(unused)]
use super::TuiState;
use zc_core::model::get_model_manager;

pub struct StateProcessor;

impl StateProcessor {
	pub fn start_prompt_run(state: &mut TuiState) {
		state.clear_input();
		state.set_waiting(true);
		state.set_last_error(None);
	}

	pub fn apply_run_start(state: &mut TuiState) {
		state.set_status("Sending to AI...".to_string());
	}

	pub fn apply_run_end(state: &mut TuiState) {
		state.set_waiting(false);
		state.set_status("Idle".to_string());
	}

	pub fn apply_run_result(state: &mut TuiState, answer: String) {
		state.set_last_answer(Some(answer));
	}

	pub fn apply_run_error(state: &mut TuiState, error: String) {
		state.set_waiting(false);
		state.set_status("Error".to_string());
		state.set_last_error(Some(error));
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
}

// endregion: --- Tests
