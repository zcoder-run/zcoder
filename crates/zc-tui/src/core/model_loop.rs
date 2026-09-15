use crate::core::event::{TuiEvent, TuiTx};
use zc_router::{CoreMsgData, CoreMsgRx, ModelChangeRx};

// region:    --- Model Loop

/// Runs the model event loop, listening to model change messages delivered by
/// `zc-router` and forwarding them to the TUI event channel.
pub async fn run_model_loop(tui_tx: TuiTx, mut model_change_rx: ModelChangeRx) {
	while let Ok(msg) = model_change_rx.recv().await {
		let CoreMsgData::ModelChange(model_event) = msg.data else {
			continue;
		};
		let res = tui_tx.send(TuiEvent::Model(model_event)).await;
		if res.is_err() {
			break;
		}
	}
}

// endregion: --- Model Loop

// region:    --- Exec Loop

/// Runs the exec event loop, listening to run lifecycle messages delivered by
/// `zc-router` and forwarding them to the TUI event channel.
pub async fn run_exec_loop(tui_tx: TuiTx, mut exec_event_rx: CoreMsgRx) {
	while let Ok(msg) = exec_event_rx.recv().await {
		let CoreMsgData::ExecEvent(exec_event) = msg.data else {
			continue;
		};
		let res = tui_tx.send(TuiEvent::Exec(exec_event)).await;
		if res.is_err() {
			break;
		}
	}
}

// endregion: --- Exec Loop

