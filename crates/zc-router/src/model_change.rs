use crate::msg::{CoreMsg, CoreMsgData};
use zc_core::model::get_model_bus;

// region:    --- Types

pub use zc_core::model::ModelChangeEvent;

// endregion: --- Types

// region:    --- Model Change Channel

/// Channel carrying model change messages from the router to a frontend.
pub type ModelChangeTx = zc_common::event_base::MpscTx<CoreMsg>;
pub type ModelChangeRx = zc_common::event_base::MpscRx<CoreMsg>;

/// Creates the bounded `CoreMsg` channel used to deliver model changes to a frontend.
pub fn new_model_change_channel() -> (ModelChangeTx, ModelChangeRx) {
	let (tx, rx) = zc_common::event_base::new_mpsc_bounded_default("model_change_channel")
		.expect("model change channel capacity is non-zero");
	(tx, rx)
}

// endregion: --- Model Change Channel

// region:    --- Model Change Loop

/// Runs the model change loop, listening to the Core model bus and forwarding each
/// change to the frontend as a `CoreMsgData::ModelChange` message.
pub async fn run_model_change_loop(model_change_tx: ModelChangeTx) {
	let mut model_rx = get_model_bus().subscribe();

	while let Ok(event) = model_rx.recv().await {
		let msg = CoreMsg::new(CoreMsgData::ModelChange(event));
		if model_change_tx.send(msg).await.is_err() {
			break;
		}
	}
}

// endregion: --- Model Change Loop
