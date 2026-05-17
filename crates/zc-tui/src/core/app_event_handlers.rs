use super::{AppActionEvent, TuiEvent, TuiState};
use crate::Result;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc::Sender;
use zc_core::{ExecActionEvent, ExecStatusEvent, ExecutorTx};

pub async fn handle_term_event(state: &mut TuiState, app_tx: &Sender<TuiEvent>, term_event: Event) {
	if let Event::Key(key) = term_event
		&& key.kind == KeyEventKind::Press
	{
		match key.code {
			KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
				let _ = app_tx.send(TuiEvent::Action(AppActionEvent::Quit)).await;
			}
			KeyCode::Enter => {
				let trimmed_input = state.input().trim().to_string();
				if trimmed_input == "/q" {
					let _ = app_tx.send(TuiEvent::Action(AppActionEvent::Quit)).await;
				} else if !trimmed_input.is_empty() && !state.is_waiting() {
					let prompt = state.input().to_string();
					let _ = app_tx.send(TuiEvent::Action(AppActionEvent::RunPrompt(prompt))).await;
				}
			}
			KeyCode::Backspace => {
				state.pop_input();
			}
			KeyCode::Char(c) => {
				state.push_input(c);
			}
			_ => {}
		}
	}
}

pub async fn handle_app_action(state: &mut TuiState, executor_tx: &ExecutorTx, action: AppActionEvent) -> Result<bool> {
	match action {
		AppActionEvent::Quit => Ok(true),
		AppActionEvent::RunPrompt(prompt) => {
			state.clear_input();
			state.set_waiting(true);
			state.set_last_error(None);
			executor_tx.send(ExecActionEvent::RunPrompt(prompt)).await?;
			Ok(false)
		}
	}
}

pub fn handle_exec_status(state: &mut TuiState, status: ExecStatusEvent) {
	match status {
		ExecStatusEvent::RunStart => {
			state.set_status("Sending to AI...".to_string());
		}
		ExecStatusEvent::RunEnd => {
			state.set_waiting(false);
			state.set_status("Idle".to_string());
		}
		ExecStatusEvent::RunResult(answer) => {
			state.set_last_answer(Some(answer));
		}
		ExecStatusEvent::RunError(err) => {
			state.set_last_error(Some(err));
		}
	}
}
