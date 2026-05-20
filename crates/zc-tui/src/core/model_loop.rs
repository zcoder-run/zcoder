use crate::event::{TuiEvent, TuiTx};
use zc_core::model::get_model_bus;

// region:    --- Model Loop

/// Runs the model event loop, listening to model events from the `zc-core` model bus
/// and forwarding them to the TUI event channel.
pub async fn run_model_loop(tui_tx: TuiTx) {
	let mut model_rx = get_model_bus().subscribe();

	while let Ok(model_event) = model_rx.recv().await {
		let res = tui_tx.send(TuiEvent::Model(model_event)).await;
		if res.is_err() {
			break;
		}
	}
}

// endregion: --- Model Loop
