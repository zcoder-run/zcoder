use crate::Result;
use crate::core::TuiState;
use crate::core::event::{AppActionEvent, TuiEvent, TuiTx};
use crate::core::tui_state::StateProcessor;
use crate::core::types::ScrollIden;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind};
use ratatui::layout::Position;
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
	let should_quit = match app_event {
		TuiEvent::Term(term_event) => {
			handle_term_event(state, tui_tx, term_event).await;
			false
		}

		TuiEvent::Action(action) => {
			if handle_app_action(state, executor_tx, action).await? {
				return Ok(true);
			}
			false
		}

		TuiEvent::Exec(status) => {
			handle_exec_status(state, status).await;
			false
		}

		TuiEvent::Model(model_event) => {
			handle_model_event(state, model_event).await?;
			false
		}

		TuiEvent::Tick | TuiEvent::DoRedraw => false,
	};

	StateProcessor::process_sys_metrics(state).await;

	Ok(should_quit)
}

pub async fn handle_term_event(state: &mut TuiState, tui_tx: &TuiTx, term_event: Event) {
	match term_event {
		Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
			KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
				let _ = tui_tx.send(TuiEvent::Action(AppActionEvent::Quit)).await;
			}
			KeyCode::F(2) => {
				state.toggle_show_sys_states();
				let _ = tui_tx.send(TuiEvent::DoRedraw).await;
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
			KeyCode::PageUp => {
				state.dec_scroll(ScrollIden::AnswerContent, 5);
			}
			KeyCode::PageDown => {
				state.inc_scroll(ScrollIden::AnswerContent, 5);
			}
			KeyCode::Home => {
				state.set_scroll(ScrollIden::AnswerContent, 0);
			}
			KeyCode::End => {
				state.set_scroll(ScrollIden::AnswerContent, u16::MAX);
			}
			KeyCode::Backspace => {
				state.pop_input();
			}
			KeyCode::Char(c) => {
				state.push_input(c);
			}
			_ => {}
		},
		Event::Mouse(mouse_event) => {
			let pos = Position::new(mouse_event.column, mouse_event.row);
			if let Some(iden) = state.scroll_zones().find_zone_for_pos(pos) {
				match mouse_event.kind {
					MouseEventKind::ScrollUp => {
						state.dec_scroll(iden, 2);
					}
					MouseEventKind::ScrollDown => {
						state.inc_scroll(iden, 2);
					}
					_ => {}
				}
			}
		}
		_ => {}
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

pub async fn handle_exec_status(state: &mut TuiState, status: ExecEvent) {
	match status {
		ExecEvent::RunStart(id) => {
			state.set_status(format!("Sending to AI (run: {id})..."));
		}
		ExecEvent::RunEnd(_id) => {
			state.set_waiting(false);
			state.set_status("Idle".to_string());
		}
		ExecEvent::RunError(id) => {
			state.set_waiting(false);
			state.set_status("Error".to_string());
			if let Ok(mm) = get_model_manager()
				&& let Ok(run) = RunBmc::get(mm, id).await
				&& let Some(err) = run.error
			{
				state.set_last_error(Some(err));
			}
		}
	}
}

pub async fn handle_model_event(state: &mut TuiState, model_event: ModelEvent) -> Result<()> {
	// do nothing for now
	// tracing::debug!("TUI GOT MODEL EVENT:\n{model_event:#?}")
	match model_event.entity {
		zc_core::model::EntityType::Run => {
			let mm = get_model_manager()?;
			if let Some(run_id) = model_event.id
				&& let Ok(run) = RunBmc::get(mm, run_id).await
			{
				state.set_last_answer(run.answer);
				if let Some(error) = run.error {
					state.set_last_error(Some(error));
				}
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

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crossterm::event::{KeyEvent, KeyEventState};
	use zc_common::event_base::new_mpsc_bounded;

	#[tokio::test]
	async fn test_core_tui_event_handlers_f2_toggle() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let (tui_tx, mut rx) = new_mpsc_bounded("test_tui", 10)?;
		let (exec_tx, _) = new_mpsc_bounded("test_exec", 10)?;

		let f2_event = TuiEvent::Term(Event::Key(KeyEvent {
			code: KeyCode::F(2),
			modifiers: KeyModifiers::empty(),
			kind: KeyEventKind::Press,
			state: KeyEventState::empty(),
		}));

		// -- Exec
		assert!(!state.show_sys_states());
		let quit = handle_tui_event(&mut state, &tui_tx, &exec_tx, f2_event.clone()).await?;

		// -- Check
		assert!(!quit);
		assert!(state.show_sys_states());
		assert!(state.memory() > 0);
		assert!(state.db_memory() > 0);
		let received = rx.recv().await?;
		assert!(matches!(received, TuiEvent::DoRedraw));

		// -- Exec (Toggle back)
		let quit = handle_tui_event(&mut state, &tui_tx, &exec_tx, f2_event).await?;

		// -- Check
		assert!(!quit);
		assert!(!state.show_sys_states());
		let received = rx.recv().await?;
		assert!(matches!(received, TuiEvent::DoRedraw));

		Ok(())
	}

	#[tokio::test]
	async fn test_core_tui_event_handlers_model_event_refreshes_sys_metrics() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.set_show_sys_states(true);
		let (tui_tx, _) = new_mpsc_bounded("test_tui", 10)?;
		let (exec_tx, _) = new_mpsc_bounded("test_exec", 10)?;

		let model_event = TuiEvent::Model(zc_core::model::ModelEvent::new(
			zc_core::model::EntityType::Run,
			zc_core::model::EntityAction::Created,
			None,
			zc_core::model::RelIds::default(),
		));

		// -- Exec
		let quit = handle_tui_event(&mut state, &tui_tx, &exec_tx, model_event).await?;

		// -- Check
		assert!(!quit);
		assert!(state.memory() > 0);
		assert!(state.db_memory() > 0);

		Ok(())
	}

	#[tokio::test]
	async fn test_core_tui_event_handlers_exec_run_error() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		state.set_waiting(true);
		state.set_status("Sending to AI...".to_string());

		let mm = get_model_manager()?;
		let run_id = RunBmc::create(
			mm,
			zc_core::model::RunForCreate {
				prompt: Some("Run error test".to_string()),
				answer: None,
			},
		)
		.await?;
		RunBmc::update(
			mm,
			run_id,
			zc_core::model::RunForUpdate {
				error: Some("Script runtime failure".to_string()),
				..Default::default()
			},
		)
		.await?;

		// -- Exec
		handle_exec_status(&mut state, ExecEvent::RunError(run_id)).await;

		// -- Check
		assert!(!state.is_waiting());
		assert_eq!(state.status(), "Error");
		assert_eq!(state.last_error(), Some("Script runtime failure"));

		Ok(())
	}

	#[tokio::test]
	async fn test_core_tui_event_handlers_model_event_with_run_error() -> Result<()> {
		// -- Setup & Fixtures
		let mut state = TuiState::new(None);
		let (tui_tx, _) = new_mpsc_bounded("test_tui", 10)?;
		let (exec_tx, _) = new_mpsc_bounded("test_exec", 10)?;

		let mm = get_model_manager()?;
		let run_id = RunBmc::create(
			mm,
			zc_core::model::RunForCreate {
				prompt: Some("Run error model event test".to_string()),
				answer: None,
			},
		)
		.await?;
		RunBmc::update(
			mm,
			run_id,
			zc_core::model::RunForUpdate {
				error: Some("Syntax error in script".to_string()),
				..Default::default()
			},
		)
		.await?;

		let model_event = TuiEvent::Model(zc_core::model::ModelEvent::new(
			zc_core::model::EntityType::Run,
			zc_core::model::EntityAction::Updated,
			Some(run_id),
			zc_core::model::RelIds::default(),
		));

		// -- Exec
		let quit = handle_tui_event(&mut state, &tui_tx, &exec_tx, model_event).await?;

		// -- Check
		assert!(!quit);
		assert_eq!(state.last_error(), Some("Syntax error in script"));

		Ok(())
	}
}

// endregion: --- Tests
