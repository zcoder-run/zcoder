use super::debouncer::debounce_events;
use super::event::{PingTimerTx, TuiRx, TuiTx};
use super::tui_event_handlers::handle_tui_event;
use crate::core::TuiState;
use crate::{Result, view};
use ratatui::DefaultTerminal;
use tracing::warn;
use zc_core::exec::ExecCmdTx;

pub async fn run_ui_loop(
	mut terminal: DefaultTerminal,
	mut tui_rx: TuiRx,
	tui_tx: TuiTx,
	ping_tx: PingTimerTx,
	executor_tx: ExecCmdTx,
	initial_prompt: Option<String>,
) -> Result<()> {
	let mut state = TuiState::new(initial_prompt);

	loop {
		terminal.draw(|f| view::render(f, &mut state))?;

		let app_event = tui_rx.recv().await?;
		let events = debounce_events(&mut tui_rx, app_event);

		let mut should_quit = false;
		for event in events {
			match handle_tui_event(&mut state, &tui_tx, &executor_tx, event).await {
				Ok(false) => (),
				Ok(true) => {
					should_quit = true;
					break;
				}
				Err(err) => {
					warn!("tui loop error on app_event. Cause: {err:?}");
					should_quit = true;
					break;
				}
			}
		}

		if should_quit {
			break;
		}

		if state.should_be_pinged() {
			let _ = ping_tx.send(zc_common::time::now_micro()).await;
		}
	}

	Ok(())
}
