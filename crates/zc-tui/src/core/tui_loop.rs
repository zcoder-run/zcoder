use super::event::{TuiRx, TuiTx};
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
	executor_tx: ExecCmdTx,
	initial_prompt: Option<String>,
) -> Result<()> {
	let mut state = TuiState::new(initial_prompt);

	loop {
		terminal.draw(|f| view::render(f, &mut state))?;

		let app_event = tui_rx.recv().await?;

		match handle_tui_event(&mut state, &tui_tx, &executor_tx, app_event).await {
			Ok(false) => (),
			Ok(true) => break,
			Err(err) => {
				warn!("tui loop error on app_event. Cause: {err:?}");
				break;
			}
		}
	}

	Ok(())
}
