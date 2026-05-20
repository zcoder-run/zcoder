use super::app_event_handlers::{handle_app_action, handle_exec_status, handle_term_event};
use crate::core::TuiState;
use crate::event::{TuiEvent, TuiRx, TuiTx};
use crate::{Result, view};
use ratatui::DefaultTerminal;
use tracing::debug;
use zc_core::exec::ExecCmdTx;
use zc_core::model::{RunBmc, get_model_manager};

pub async fn run_ui_loop(
	mut terminal: DefaultTerminal,
	mut tui_rx: TuiRx,
	tui_tx: TuiTx,
	executor_tx: ExecCmdTx,
	initial_prompt: Option<String>,
) -> Result<()> {
	let mut state = TuiState::new(initial_prompt);
	let mm = get_model_manager()?;

	loop {
		terminal.draw(|f| view::render(f, &state))?;

		let app_event = tui_rx.recv().await?;

		match app_event {
			TuiEvent::Term(term_event) => {
				handle_term_event(&mut state, &tui_tx, term_event).await;
			}

			TuiEvent::Action(action) => {
				if handle_app_action(&mut state, &executor_tx, action).await? {
					break;
				}
			}

			TuiEvent::Exec(status) => {
				handle_exec_status(&mut state, status);
			}

			TuiEvent::Model(model_event) => {
				// do nothing for now
				// tracing::debug!("TUI GOT MODEL EVENT:\n{model_event:#?}")
				match model_event.entity {
					zc_core::model::EntityType::Run => {
						if let Some(run_id) = model_event.id
							&& let Ok(run) = RunBmc::get(mm, run_id)
						{
							state.set_last_answer(run.answer);
						} else {
							debug!("Error while model event (tui)")
						}
					}
				}
			}

			TuiEvent::Tick | TuiEvent::DoRedraw => (),
		}
	}

	Ok(())
}
