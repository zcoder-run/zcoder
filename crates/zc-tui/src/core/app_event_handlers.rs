use crate::Result;
use crate::core::TuiState;
use crate::event::{AppActionEvent, TuiEvent, TuiTx};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use zc_core::exec::{ExecCmd, ExecCmdTx, ExecEvent};

pub async fn handle_term_event(state: &mut TuiState, tui_tx: &TuiTx, term_event: Event) {
	if let Event::Key(key) = term_event
		&& key.kind == KeyEventKind::Press
	{
		match key.code {
			KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
				let _ = tui_tx.send(TuiEvent::Action(AppActionEvent::Quit)).await;
			}
			KeyCode::Enter => {
				let trimmed_input = state.input().trim().to_string();
				if trimmed_input == "/q" {
					let _ = tui_tx.send(TuiEvent::Action(AppActionEvent::Quit)).await;
				} else if !trimmed_input.is_empty() && !state.is_waiting() {
					let prompt = state.input().to_string();
					let _ = tui_tx.send(TuiEvent::Action(AppActionEvent::RunPrompt(prompt))).await;
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

pub async fn handle_app_action(state: &mut TuiState, executor_tx: &ExecCmdTx, action: AppActionEvent) -> Result<bool> {
	match action {
		AppActionEvent::Quit => Ok(true),
		AppActionEvent::RunPrompt(prompt) => {
			state.clear_input();
			state.set_waiting(true);
			state.set_last_error(None);
			executor_tx.send(ExecCmd::RunPrompt(prompt)).await?;
			Ok(false)
		}
	}
}

pub fn handle_exec_status(state: &mut TuiState, status: ExecEvent) {
	match status {
		ExecEvent::RunStart(id) => {
			state.set_status(format!("Sending to AI (run: {id})..."));
		}
		ExecEvent::RunEnd(_id) => {
			state.set_waiting(false);
			state.set_status("Idle".to_string());
		}
		ExecEvent::RunError(_id) => {
			// TODO: Get error
			// state.set_last_error(Some(err));
		}
	}
}
