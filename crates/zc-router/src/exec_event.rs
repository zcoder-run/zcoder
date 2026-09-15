// region:    --- Modules

use crate::msg::{CoreMsg, CoreMsgData, CoreMsgRx, CoreMsgTx};
use zc_core::exec::ExecEventRx;

pub use zc_core::exec::ExecEvent;

// endregion: --- Modules

// region:    --- Exec Event Channel

/// Creates the outbound `CoreMsg` channel used to deliver run lifecycle events
/// from the router to a frontend such as the TUI.
pub fn new_exec_event_channel() -> (CoreMsgTx, CoreMsgRx) {
	let (tx, rx) = zc_common::event_base::new_mpsc_bounded_default("exec_event_channel")
		.expect("exec event channel capacity is non-zero");
	(tx, rx)
}

// endregion: --- Exec Event Channel

// region:    --- Exec Event Loop

/// Reads run lifecycle events from the Core executor status stream and forwards
/// each one as a `CoreMsgData::ExecEvent` on the outbound Core message channel.
pub async fn run_exec_event_loop(mut exec_event_rx: ExecEventRx, exec_event_tx: CoreMsgTx) {
	while let Ok(exec_event) = exec_event_rx.recv().await {
		let msg = CoreMsg::new(CoreMsgData::ExecEvent(exec_event));
		if exec_event_tx.send(msg).await.is_err() {
			break;
		}
	}
}

// endregion: --- Exec Event Loop
