use crate::exec::ExecCmd;
use crate::exec_event::ExecEvent;
use crate::model_change::ModelChangeEvent;
use crate::model_rpc::ModelRpcCmd;
use std::sync::atomic::{AtomicU64, Ordering};
use zc_common::MsgId;
use zc_core::model::Id;

// region:    --- Types

/// Generic envelope carrying messages between frontends (TUI, base) and Core.
#[derive(Debug)]
pub struct RouterMsg {
	pub msg_id: MsgId,
	pub wks_id: Id,
	pub data: RouterMsgData,
}

/// Payload carried by a [`RouterMsg`]. The variants are intentionally not all the same semantic kind.
#[derive(Debug)]
pub enum RouterMsgData {
	ModelRpc(ModelRpcCmd),
	ModelChange(ModelChangeEvent),
	Exec(ExecCmd),
	ExecEvent(ExecEvent),
}

// endregion: --- Types

// region:    --- RouterMsg Constructors

/// Monotonic source of `MsgId` values for messages created in this process.
static NEXT_MSG_ID: AtomicU64 = AtomicU64::new(1);

impl RouterMsg {
	/// Creates a message with a fresh [`MsgId`] and the single-workspace stub `wks_id`.
	pub fn new(data: RouterMsgData) -> Self {
		let msg_id = MsgId::new(NEXT_MSG_ID.fetch_add(1, Ordering::Relaxed));
		Self {
			msg_id,
			wks_id: Id::default(),
			data,
		}
	}
}

// endregion: --- RouterMsg Constructors

// region:    --- RouterMsg Channels

pub type RouterMsgTx = zc_common::event_base::MpscTx<RouterMsg>;
pub type RouterMsgRx = zc_common::event_base::MpscRx<RouterMsg>;

/// Creates the bounded `RouterMsg` channel used between frontends and the router.
pub fn new_router_msg_channel() -> (RouterMsgTx, RouterMsgRx) {
	let (tx, rx) = zc_common::event_base::new_mpsc_bounded_default("router_msg_channel")
		.expect("core msg channel capacity is non-zero");
	(tx, rx)
}

// endregion: --- RouterMsg Channels

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_msg_router_msg_construct() -> Result<()> {
		// -- Exec
		let msg = RouterMsg {
			msg_id: MsgId::new(7),
			wks_id: Id::default(),
			data: RouterMsgData::Exec(ExecCmd::RunPrompt("hello".to_string())),
		};

		// -- Check
		assert_eq!(msg.msg_id.as_u64(), 7);
		assert!(matches!(msg.data, RouterMsgData::Exec(_)));

		Ok(())
	}
}

// endregion: --- Tests
