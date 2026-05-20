use super::{term_reader, tui_loop};
use crate::Result;
use crate::core::model_loop::run_model_loop;
use crate::event::TuiEvent;
use zc_common::event::new_mpsc_bounded;
use zc_core::exec::{ExecCmdTx, ExecEventRx};

pub async fn start_tui(
	executor_tx: ExecCmdTx,
	mut status_rx: ExecEventRx,
	initial_prompt: Option<String>,
) -> Result<()> {
	// -- Init Terminal
	let mut terminal = ratatui::init();
	terminal.clear()?;

	// -- Create AppEvent channels
	let (tui_tx, tui_rx) = new_mpsc_bounded::<TuiEvent>();

	// -- Run the model loop
	let tui_tx_for_model = tui_tx.clone();
	tokio::spawn(async move { run_model_loop(tui_tx_for_model).await });

	// -- Spawn status event forwarder
	let tui_tx_for_exec = tui_tx.clone();
	tokio::spawn(async move {
		while let Ok(status) = status_rx.recv().await {
			if tui_tx_for_exec.send(TuiEvent::Exec(status)).await.is_err() {
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
