use crate::model::{RunBmc, get_model_manager};
use zc_core::exec::ExecEventRx;
use zc_core::model::Id;
use zc_router::{RouterMsg, RouterMsgData, RouterMsgTx};

/// Reads run lifecycle events from the Core executor status stream and forwards
/// each one as a `RouterMsgData::ExecEvent` on the outbound Core message channel.
pub async fn run_exec_event_loop(mut exec_event_rx: ExecEventRx, exec_event_tx: RouterMsgTx) {
	while let Ok(exec_event) = exec_event_rx.recv().await {
		let run_id = match &exec_event {
			zc_core::exec::ExecEvent::RunStart(id) => *id,
			zc_core::exec::ExecEvent::RunEnd(id) => *id,
			zc_core::exec::ExecEvent::RunError(id) => *id,
		};
		let wks_id = if let Ok(mm) = get_model_manager()
			&& let Ok(run) = RunBmc::get(mm, run_id).await
			&& let Some(wks_id) = run.wks_id
		{
			wks_id
		} else {
			Id::default()
		};
		let mut msg = RouterMsg::new(RouterMsgData::ExecEvent(exec_event));
		msg.wks_id = wks_id;
		if exec_event_tx.send(msg).await.is_err() {
			break;
		}
	}
}
