use crate::client_info::ClientInfo;
use crate::error::Result;
use crate::exec::ExecCmd;
use crate::exec_event::ExecEvent;
use crate::model_change::ModelChangeEvent;
use crate::model_rpc::{ModelRpcCmdTx, ModelRpcReply, ModelRpcReq};
use crate::msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx};
use zc_common::MsgId;
use zc_core::exec::{ExecCmdTx, ExecReq};
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
	let wspace_id = msg.wspace_id;

	match msg.data {
		RouterMsgData::Exec(cmd) => {
			route_exec(exec_cmd_tx, msg_id, wspace_id, cmd).await?;
		}
		RouterMsgData::ModelRpcReq(req) => {
			route_model_rpc_req(model_rpc_cmd_tx, reply_tx, msg_id, wspace_id, req).await?;
		}
		RouterMsgData::ModelRpcRes(reply) => {
			route_model_rpc_res(reply_tx, msg_id, wspace_id, reply).await?;
		}
		RouterMsgData::ModelChange(event) => {
			route_model_change(msg_id, wspace_id, event).await?;
		}
		RouterMsgData::ExecEvent(event) => {
			route_exec_event(msg_id, wspace_id, event).await?;
		}
		RouterMsgData::Attach(info) => {
			route_attach(msg_id, wspace_id, info).await?;
		}
		RouterMsgData::AttachOk(assigned) => {
			route_attach_ok(msg_id, wspace_id, assigned).await?;
		}
		RouterMsgData::AttachErr(message) => {
			route_attach_err(msg_id, wspace_id, message).await?;
		}
	}

	Ok(())
}

// endregion: --- Router

// region:    --- Support

async fn route_exec(exec_cmd_tx: &ExecCmdTx, msg_id: MsgId, wspace_id: Id, cmd: ExecCmd) -> Result<()> {
	tracing::debug!("->> route_exec msg_id={msg_id:?} wspace_id={wspace_id:?} cmd={cmd:?}");
	exec_cmd_tx.send(ExecReq { wspace_id, cmd }).await?;
	Ok(())
}

async fn route_model_change(msg_id: MsgId, wspace_id: Id, event: ModelChangeEvent) -> Result<()> {
	tracing::debug!("->> route_model_change msg_id={msg_id:?} wspace_id={wspace_id:?} event={event:?}");
	Ok(())
}

async fn route_exec_event(msg_id: MsgId, wspace_id: Id, event: ExecEvent) -> Result<()> {
	tracing::debug!("->> route_exec_event msg_id={msg_id:?} wspace_id={wspace_id:?} event={event:?}");
	Ok(())
}

async fn route_model_rpc_req(
	model_rpc_cmd_tx: &ModelRpcCmdTx,
	reply_tx: &RouterMsgTx,
	msg_id: MsgId,
	wspace_id: Id,
	req: ModelRpcReq,
) -> Result<()> {
	tracing::debug!("->> route_model_rpc_req msg_id={msg_id:?} wspace_id={wspace_id:?} req={req:?}");
	let (res_tx, res_rx) = zc_common::event_base::new_once("model_rpc_local");
	let cmd = req.into_cmd(res_tx);
	model_rpc_cmd_tx.send(cmd).await?;
	let reply_tx = reply_tx.clone();

	tokio::spawn(async move {
		if let Ok(reply) = res_rx.recv().await {
			let res_msg = RouterMsg {
				msg_id,
				wspace_id,
				data: RouterMsgData::ModelRpcRes(reply),
			};
			if reply_tx.send(res_msg).await.is_err() {
				tracing::warn!("->> failed to route model RPC reply msg_id={msg_id:?}");
			}
		}
	});

	Ok(())
}

async fn route_model_rpc_res(reply_tx: &RouterMsgTx, msg_id: MsgId, wspace_id: Id, reply: ModelRpcReply) -> Result<()> {
	tracing::debug!("->> route_model_rpc_res msg_id={msg_id:?} wspace_id={wspace_id:?} reply={reply:?}");
	reply_tx
		.send(RouterMsg {
			msg_id,
			wspace_id,
			data: RouterMsgData::ModelRpcRes(reply),
		})
		.await?;
	Ok(())
}

async fn route_attach(msg_id: MsgId, wspace_id: Id, info: ClientInfo) -> Result<()> {
	tracing::debug!("->> route_attach msg_id={msg_id:?} wspace_id={wspace_id:?} info={info:?}");
	Ok(())
}

async fn route_attach_ok(msg_id: MsgId, wspace_id: Id, assigned: Id) -> Result<()> {
	tracing::debug!("->> route_attach_ok msg_id={msg_id:?} wspace_id={wspace_id:?} assigned={assigned:?}");
	Ok(())
}

async fn route_attach_err(msg_id: MsgId, wspace_id: Id, message: String) -> Result<()> {
	tracing::debug!("->> route_attach_err msg_id={msg_id:?} wspace_id={wspace_id:?} message={message}");
	Ok(())
}

// endregion: --- Support
