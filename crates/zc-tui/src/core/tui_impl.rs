use super::{TuiEvent, term_reader, tui_loop};
use crate::Result;
use tokio::sync::mpsc::{self, Sender};
use zc_core::{ExecutorStatusRx, ExecutorTx};

pub type AppTx = Sender<TuiEvent>;

pub async fn start_tui(
	executor_tx: ExecutorTx,
	status_rx: ExecutorStatusRx,
	initial_prompt: Option<String>,
) -> Result<()> {
	// -- Init Terminal
	let mut terminal = ratatui::init();
	terminal.clear()?;

	// -- Create AppEvent channels
	let (app_tx, app_rx) = mpsc::channel(100);

	// -- Spawn status event forwarder
	let app_tx_for_status = app_tx.clone();
	tokio::spawn(async move {
		while let Ok(status) = status_rx.recv().await {
			if app_tx_for_status.send(TuiEvent::Exec(status)).await.is_err() {
				break;
			}
		}
	});

	// -- Start Term Reader
	term_reader::run_term_reader(app_tx.clone());

	// -- Start TUI Loop
	let res = tui_loop::run_ui_loop(terminal, app_rx, app_tx, executor_tx, initial_prompt).await;

	// -- Restore Terminal
	ratatui::restore();

	res
}
