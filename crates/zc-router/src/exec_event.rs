// region:    --- Modules

use crate::msg::{RouterMsgRx, RouterMsgTx};

pub use zc_core::exec::ExecEvent;

pub type ExecEventRx = RouterMsgRx;
pub type ExecEventTx = RouterMsgTx;

// endregion: --- Modules

// region:    --- Exec Event Channel

/// Creates the outbound `RouterMsg` channel used to deliver run lifecycle events
/// from the router to a frontend such as the TUI.
pub fn new_exec_event_channel() -> (RouterMsgTx, RouterMsgRx) {
	let (tx, rx) = zc_common::event_base::new_mpsc_bounded_default("exec_event_channel")
		.expect("exec event channel capacity is non-zero");
	(tx, rx)
}

// endregion: --- Exec Event Channel
