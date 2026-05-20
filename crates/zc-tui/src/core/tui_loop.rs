use super::app_event_handlers::{handle_app_action, handle_exec_status, handle_term_event};
use crate::core::TuiState;
use crate::event::{TuiEvent, TuiRx, TuiTx};
use crate::{Result, view};
use ratatui::DefaultTerminal;
use zc_core::exec::ExecActionTx;

pub async fn run_ui_loop(
	mut terminal: DefaultTerminal,
	mut tui_rx: TuiRx,
	tui_tx: TuiTx,
	executor_tx: ExecActionTx,
	initial_prompt: Option<String>,
) -> Result<()> {
	let mut state = TuiState::new(initial_prompt);

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

			TuiEvent::Model(_model_event) => {
				// do nothing for now
			}

			TuiEvent::Tick | TuiEvent::DoRedraw => (),
		}
	}

	Ok(())
}
