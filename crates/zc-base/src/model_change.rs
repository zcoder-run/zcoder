// region:    --- Modules

use crate::model::get_model_bus;
use zc_router::{ModelChangeTx, RouterMsg, RouterMsgData};

// endregion: --- Modules

// region:    --- Model Change Loop

/// Runs the model change loop, listening to the Core model bus and forwarding each
/// change to the frontend as a `RouterMsgData::ModelChange` message.
pub async fn run_model_change_loop(model_change_tx: ModelChangeTx) {
	let mut model_rx = get_model_bus().subscribe();

	while let Ok(event) = model_rx.recv().await {
		let msg = RouterMsg::new(RouterMsgData::ModelChange(event));
		if model_change_tx.send(msg).await.is_err() {
			break;
		}
	}
}

// endregion: --- Model Change Loop
