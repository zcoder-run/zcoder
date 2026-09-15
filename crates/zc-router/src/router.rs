use crate::error::Result;
use crate::exec::ExecCmd;
use crate::model_rpc::ModelRpcCmd;
use crate::msg::{CoreMsg, CoreMsgData, CoreMsgRx};
use zc_common::MsgId;
use zc_core::exec::ExecCmdTx;
use zc_core::model::Id;

// region:    --- Router Loop

/// Receives [`CoreMsg`] values from the transport and routes each one to Core.
pub async fn run_router(mut router_rx: CoreMsgRx, exec_cmd_tx: ExecCmdTx) -> Result<()> {
	while let Ok(msg) = router_rx.recv().await {
		route(&exec_cmd_tx, msg).await?;
	}

	Ok(())
}

// endregion: --- Router Loop

// region:    --- Router

/// Dispatches a [`CoreMsg`] to the appropriate Core subsystem.
pub async fn route(exec_cmd_tx: &ExecCmdTx, msg: CoreMsg) -> Result<()> {
	let msg_id = msg.msg_id;
	let wks_id = msg.wks_id;

	match msg.data {
		CoreMsgData::Exec(cmd) => {
			route_exec(exec_cmd_tx, msg_id, wks_id, cmd).await?;
		}
		CoreMsgData::ModelRpc(cmd) => {
			route_model_rpc(msg_id, wks_id, cmd).await?;
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

async fn route_model_rpc(msg_id: MsgId, wks_id: Id, cmd: ModelRpcCmd) -> Result<()> {
	tracing::debug!("->> route_model_rpc msg_id={msg_id:?} wks_id={wks_id:?} cmd={cmd:?}");
	Ok(())
}

// endregion: --- Support
