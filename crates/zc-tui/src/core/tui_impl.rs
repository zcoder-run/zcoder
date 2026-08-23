use super::event::TuiEvent;
use super::{ping_timer, term_reader, tui_loop};
use crate::Result;
use crate::core::model_loop::run_model_loop;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use std::io::stdout;
use zc_common::event_base::new_mpsc_bounded;
use zc_core::exec::{ExecCmdTx, ExecEventRx};

pub async fn start_tui(executor_tx: ExecCmdTx, mut exec_rx: ExecEventRx, initial_prompt: Option<String>) -> Result<()> {
	// -- Init Terminal
	let mut terminal = ratatui::init();
	execute!(stdout(), EnableMouseCapture)?;
	terminal.clear()?;

	// -- Create AppEvent channels
	let (tui_tx, tui_rx) = new_mpsc_bounded::<TuiEvent>("tui_channel", 1000)?;

	// -- Run the model loop
	let tui_tx_for_model = tui_tx.clone();
	tokio::spawn(async move { run_model_loop(tui_tx_for_model).await });

	// -- Spawn status event forwarder
	let tui_tx_for_exec = tui_tx.clone();
	tokio::spawn(async move {
		while let Ok(status) = exec_rx.recv().await {
			if tui_tx_for_exec.send(TuiEvent::Exec(status)).await.is_err() {
				break;
			}
		}
	});

	// -- Start Term Reader
	term_reader::run_term_reader(tui_tx.clone());

	// -- Start Ping Timer
	let ping_tx = ping_timer::start_ping_timer(tui_tx.clone())?;

	// -- Start TUI Loop
	let res = tui_loop::run_ui_loop(terminal, tui_rx, tui_tx, ping_tx, executor_tx, initial_prompt).await;

	// -- Restore Terminal
	ratatui::restore();
	let _ = execute!(stdout(), DisableMouseCapture);

	res
}
