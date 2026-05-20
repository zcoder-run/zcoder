use super::{term_reader, tui_loop};
use crate::Result;
use crate::event::TuiEvent;
use zc_common::event::new_mpsc_bounded;
use zc_core::exec::{ExecActionTx, ExecutorStatusRx};

pub async fn start_tui(
	executor_tx: ExecActionTx,
	mut status_rx: ExecutorStatusRx,
	initial_prompt: Option<String>,
) -> Result<()> {
	// -- Init Terminal
	let mut terminal = ratatui::init();
	terminal.clear()?;

	// -- Create AppEvent channels
	let (tui_tx, tui_rx) = new_mpsc_bounded::<TuiEvent>();

	// -- Spawn status event forwarder
	let tui_tx_for_status = tui_tx.clone();
	tokio::spawn(async move {
		while let Ok(status) = status_rx.recv().await {
			if tui_tx_for_status.send(TuiEvent::Exec(status)).await.is_err() {
				break;
			}
		}
	});

	// -- Start Term Reader
	term_reader::run_term_reader(tui_tx.clone());

	// -- Start TUI Loop
	let res = tui_loop::run_ui_loop(terminal, tui_rx, tui_tx, executor_tx, initial_prompt).await;

	// -- Restore Terminal
	ratatui::restore();

	res
}
