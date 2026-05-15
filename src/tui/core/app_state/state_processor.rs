use super::AppState;

pub struct StateProcessor;

impl StateProcessor {
	pub fn start_prompt_run(state: &mut AppState) {
		state.clear_input();
		state.set_waiting(true);
		state.set_last_error(None);
	}

	pub fn apply_run_start(state: &mut AppState) {
		state.set_status("Sending to AI...".to_string());
	}

	pub fn apply_run_end(state: &mut AppState) {
		state.set_waiting(false);
		state.set_status("Idle".to_string());
	}

	pub fn apply_run_result(state: &mut AppState, answer: String) {
		state.set_last_answer(Some(answer));
	}

	pub fn apply_run_error(state: &mut AppState, error: String) {
		state.set_last_error(Some(error));
	}
}
