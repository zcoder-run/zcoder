use crate::error::Result;
use crate::exec::ExecCmd;
use crate::exec_event::ExecEvent;
use crate::model_change::ModelChangeEvent;
use crate::model_rpc::{ModelRpcCmdTx, ModelRpcReply, ModelRpcReq};
use crate::msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx};
use zc_common::MsgId;
use zc_core::exec::ExecCmdTx;
use zc_core::model::Id;

// region:    --- Router Loop

/// Receives [`RouterMsg`] values from the transport and routes each one to Core.
pub async fn run_router(
	mut router_rx: RouterMsgRx,
	exec_cmd_tx: ExecCmdTx,
	model_rpc_cmd_tx: ModelRpcCmdTx,
	reply_tx: RouterMsgTx,
) -> Result<()> {
	while let Ok(msg) = router_rx.recv().await {
		route(&exec_cmd_tx, &model_rpc_cmd_tx, &reply_tx, msg).await?;
	}

	Ok(())
}

// endregion: --- Router Loop

// region:    --- Router

/// Dispatches a [`RouterMsg`] to the appropriate Core subsystem.
pub async fn route(
	exec_cmd_tx: &ExecCmdTx,
	model_rpc_cmd_tx: &ModelRpcCmdTx,
	reply_tx: &RouterMsgTx,
	msg: RouterMsg,
) -> Result<()> {
	let msg_id = msg.msg_id;
	let wks_id = msg.wks_id;

	match msg.data {
		RouterMsgData::Exec(cmd) => {
			route_exec(exec_cmd_tx, msg_id, wks_id, cmd).await?;
		}
		RouterMsgData::ModelRpcReq(req) => {
			route_model_rpc_req(model_rpc_cmd_tx, reply_tx, msg_id, wks_id, req).await?;
		}
		RouterMsgData::ModelRpcRes(reply) => {
			route_model_rpc_res(reply_tx, msg_id, wks_id, reply).await?;
		}
		RouterMsgData::ModelChange(event) => {
			route_model_change(msg_id, wks_id, event).await?;
		}
		RouterMsgData::ExecEvent(event) => {
			route_exec_event(msg_id, wks_id, event).await?;
		}
	}

	Ok(())
}

// endregion: --- Router

// region:    --- Support

async fn route_exec(exec_cmd_tx: &ExecCmdTx, msg_id: MsgId, wks_id: Id, cmd: ExecCmd) -> Result<()> {
	tracing::debug!("->> route_exec msg_id={msg_id:?} wks_id={wks_id:?} cmd={cmd:?}");
	exec_cmd_tx.send(cmd).await?;
	Ok(())
}

async fn route_model_change(msg_id: MsgId, wks_id: Id, event: ModelChangeEvent) -> Result<()> {
	tracing::debug!("->> route_model_change msg_id={msg_id:?} wks_id={wks_id:?} event={event:?}");
	Ok(())
}

async fn route_exec_event(msg_id: MsgId, wks_id: Id, event: ExecEvent) -> Result<()> {
	tracing::debug!("->> route_exec_event msg_id={msg_id:?} wks_id={wks_id:?} event={event:?}");
	Ok(())
}

async fn route_model_rpc_req(
	model_rpc_cmd_tx: &ModelRpcCmdTx,
	reply_tx: &RouterMsgTx,
	msg_id: MsgId,
	wks_id: Id,
	req: ModelRpcReq,
) -> Result<()> {
	tracing::debug!("->> route_model_rpc_req msg_id={msg_id:?} wks_id={wks_id:?} req={req:?}");
	let (res_tx, res_rx) = zc_common::event_base::new_once("model_rpc_local");
	let cmd = req.into_cmd(res_tx);
	model_rpc_cmd_tx.send(cmd).await?;
	let reply_tx = reply_tx.clone();

	tokio::spawn(async move {
		if let Ok(reply) = res_rx.recv().await {
			let res_msg = RouterMsg {
				msg_id,
				wks_id,
				data: RouterMsgData::ModelRpcRes(reply),
			};
			if reply_tx.send(res_msg).await.is_err() {
				tracing::warn!("->> failed to route model RPC reply msg_id={msg_id:?}");
			}
		}
	});

	Ok(())
}

async fn route_model_rpc_res(
	reply_tx: &RouterMsgTx,
	msg_id: MsgId,
	wks_id: Id,
	reply: ModelRpcReply,
) -> Result<()> {
	tracing::debug!("->> route_model_rpc_res msg_id={msg_id:?} wks_id={wks_id:?} reply={reply:?}");
	reply_tx
		.send(RouterMsg {
			msg_id,
			wks_id,
			data: RouterMsgData::ModelRpcRes(reply),
		})
		.await?;
	Ok(())
}

// endregion: --- Support
