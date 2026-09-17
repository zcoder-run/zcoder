use zc_core::exec::ExecEventRx;
use zc_router::{RouterMsg, RouterMsgData, RouterMsgTx};

/// Reads run lifecycle events from the Core executor status stream and forwards
/// each one as a `RouterMsgData::ExecEvent` on the outbound Core message channel.
pub async fn run_exec_event_loop(mut exec_event_rx: ExecEventRx, exec_event_tx: RouterMsgTx) {
	while let Ok(exec_event) = exec_event_rx.recv().await {
		let msg = RouterMsg::new(RouterMsgData::ExecEvent(exec_event));
		if exec_event_tx.send(msg).await.is_err() {
			break;
		}
	}
}
