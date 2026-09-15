use crate::error::Result;
use crate::exec::ExecCmd;
use crate::exec_event::ExecEvent;
use crate::model_change::ModelChangeEvent;
use crate::model_rpc::{ModelRpcCmd, ModelRpcError, ModelRpcReply, ModelRpcResult};
use crate::msg::{CoreMsg, CoreMsgData, CoreMsgRx};
use zc_common::MsgId;
use zc_core::exec::ExecCmdTx;
use zc_core::model::{AirBmc, Id, ModelManager, RunBmc, get_model_manager};

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
		CoreMsgData::ModelChange(event) => {
			route_model_change(msg_id, wks_id, event).await?;
		}
		CoreMsgData::ExecEvent(event) => {
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

async fn route_model_rpc(msg_id: MsgId, wks_id: Id, cmd: ModelRpcCmd) -> Result<()> {
	tracing::debug!("->> route_model_rpc msg_id={msg_id:?} wks_id={wks_id:?} cmd={cmd:?}");

	match cmd {
		ModelRpcCmd::RunGet { id, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => RunBmc::get(mm, id).await.map(Some).map_err(ModelRpcError::from),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::Run(reply));
		}
		ModelRpcCmd::RunList { options, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => RunBmc::list(mm, Some(options)).await.map_err(ModelRpcError::from),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::RunList(reply));
		}
		ModelRpcCmd::AirGet { id, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => AirBmc::get(mm, id).await.map(Some).map_err(ModelRpcError::from),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::Air(reply));
		}
		ModelRpcCmd::AirList { options, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => AirBmc::list(mm, Some(options)).await.map_err(ModelRpcError::from),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::AirList(reply));
		}
	}

	Ok(())
}

fn model_manager() -> ModelRpcResult<&'static ModelManager> {
	get_model_manager().map_err(ModelRpcError::custom)
}

// endregion: --- Support
