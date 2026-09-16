use crate::msg::RouterMsg;
// region:    --- Types
pub use zc_core::model::ModelChangeEvent;

// endregion: --- Types

// region:    --- Model Change Channel

/// Channel carrying model change messages from the router to a frontend.
pub type ModelChangeTx = zc_common::event_base::MpscTx<RouterMsg>;
pub type ModelChangeRx = zc_common::event_base::MpscRx<RouterMsg>;

/// Creates the bounded `RouterMsg` channel used to deliver model changes to a frontend.
pub fn new_model_change_channel() -> (ModelChangeTx, ModelChangeRx) {
	let (tx, rx) = zc_common::event_base::new_mpsc_bounded_default("model_change_channel")
		.expect("model change channel capacity is non-zero");
	(tx, rx)
}

// endregion: --- Model Change Channel
