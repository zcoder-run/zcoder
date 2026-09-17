use crate::model::get_model_bus;
use zc_router::{ModelChangeTx, RouterMsg, RouterMsgData};

/// Runs the model change loop, listening to the Core model bus and forwarding each
/// change to the frontend as a `RouterMsgData::ModelChange` message.
pub async fn run_model_change_loop(model_change_tx: ModelChangeTx) {
	let mut model_rx = get_model_bus().subscribe();

	while let Ok(event) = model_rx.recv().await {
		let wks_id = event.rel_ids.wks_id.unwrap_or_default();
		let mut msg = RouterMsg::new(RouterMsgData::ModelChange(event));
		msg.wks_id = wks_id;
		if model_change_tx.send(msg).await.is_err() {
			break;
		}
	}
}
