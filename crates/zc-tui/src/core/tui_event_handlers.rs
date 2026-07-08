use crate::Result;
use crate::core::TuiState;
use crate::core::event::{AppActionEvent, TuiEvent, TuiTx};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tracing::debug;
use zc_core::exec::{ExecCmd, ExecCmdTx, ExecEvent};
use zc_core::model::{ModelEvent, RunBmc, get_model_manager};

/// return `true` if needs quit
pub async fn handle_tui_event(
	state: &mut TuiState,
	tui_tx: &TuiTx,
	executor_tx: &ExecCmdTx,
	app_event: TuiEvent,
) -> Result<bool> {
	match app_event {
		TuiEvent::Term(term_event) => {
			handle_term_event(state, tui_tx, term_event).await;
		}

		TuiEvent::Action(action) => {
			if handle_app_action(state, executor_tx, action).await? {
				return Ok(true);
			}
		}

		TuiEvent::Exec(status) => {
			handle_exec_status(state, status);
		}

		TuiEvent::Model(model_event) => {
			handle_model_event(state, model_event)?;
		}

		TuiEvent::Tick | TuiEvent::DoRedraw => {}
	}
	Ok(false)
}

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

pub fn handle_model_event(state: &mut TuiState, model_event: ModelEvent) -> Result<()> {
	// do nothing for now
	// tracing::debug!("TUI GOT MODEL EVENT:\n{model_event:#?}")
	match model_event.entity {
		zc_core::model::EntityType::Run => {
			let mm = get_model_manager()?;
			if let Some(run_id) = model_event.id
				&& let Ok(run) = RunBmc::get(mm, run_id)
			{
				state.set_last_answer(run.answer);
			} else {
				debug!("Error while model event (tui)")
			}
		}
		zc_core::model::EntityType::Aixc => {
			// do nothing for now
		}
	}
	Ok(())
}
