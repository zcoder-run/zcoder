use super::app_event_handlers::{handle_app_action, handle_exec_status, handle_term_event};
use super::{AppEvent, AppState};
use crate::exec::ExecutorTx;
use crate::tui::view;
use crate::{Error, Result};
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::{Receiver, Sender};

pub async fn run_ui_loop(
	mut terminal: DefaultTerminal,
	mut app_rx: Receiver<AppEvent>,
	app_tx: Sender<AppEvent>,
	executor_tx: ExecutorTx,
	initial_prompt: Option<String>,
) -> Result<()> {
	let mut state = AppState::new(initial_prompt);

	loop {
		terminal.draw(|f| view::render(f, &state))?;

		let app_event = app_rx.recv().await.ok_or_else(|| Error::custom("App event channel closed"))?;

		match app_event {
			AppEvent::Term(term_event) => {
				handle_term_event(&mut state, &app_tx, term_event).await;
			}

			AppEvent::Action(action) => {
				if handle_app_action(&mut state, &executor_tx, action).await? {
					break;
				}
			}

			AppEvent::Exec(status) => {
				handle_exec_status(&mut state, status);
			}

			AppEvent::Tick | AppEvent::DoRedraw => (),
		}
	}

	Ok(())
}
