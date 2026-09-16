// region:    --- Modules

use zc_base::model::{AirBmc, RunBmc, get_model_manager};
use zc_core::exec::ExecCmdTx;
use zc_router::{
	ModelRpcCmd, ModelRpcCmdRx, ModelRpcError, ModelRpcReply, RouterMsgRx, new_model_rpc_cmd_channel,
	run_router,
};

// endregion: --- Modules

// region:    --- Test Router Support

/// Starts a router wired to a local model RPC stub for the `zc-tui` tests.
///
/// `zc-tui` does not own the model layer, so the router forwards its model RPC
/// commands to the stub below, which serves them by reading the model manager.
/// This mirrors what `zc-base` does in production and keeps the tests runnable
/// without a base process.
pub fn start_router_with_stub(router_msg_rx: RouterMsgRx, exec_cmd_tx: ExecCmdTx) {
	let (model_rpc_cmd_tx, model_rpc_cmd_rx) = new_model_rpc_cmd_channel();
	tokio::spawn(run_model_rpc_stub(model_rpc_cmd_rx));
	tokio::spawn(run_router(router_msg_rx, exec_cmd_tx, model_rpc_cmd_tx));
}

// endregion: --- Test Router Support

// region:    --- Support

async fn run_model_rpc_stub(mut model_rpc_cmd_rx: ModelRpcCmdRx) {
	while let Ok(cmd) = model_rpc_cmd_rx.recv().await {
		match cmd {
			ModelRpcCmd::RunGet { id, res_tx } => {
				let reply = match get_model_manager() {
					Ok(mm) => RunBmc::get(mm, id).await.map(Some).map_err(map_model_err),
					Err(err) => Err(map_model_err(err)),
				};
				res_tx.send(ModelRpcReply::Run(reply));
			}
			ModelRpcCmd::RunList { options, res_tx } => {
				let reply = match get_model_manager() {
					Ok(mm) => RunBmc::list(mm, Some(options)).await.map_err(map_model_err),
					Err(err) => Err(map_model_err(err)),
				};
				res_tx.send(ModelRpcReply::RunList(reply));
			}
			ModelRpcCmd::AirGet { id, res_tx } => {
				let reply = match get_model_manager() {
					Ok(mm) => AirBmc::get(mm, id).await.map(Some).map_err(map_model_err),
					Err(err) => Err(map_model_err(err)),
				};
				res_tx.send(ModelRpcReply::Air(reply));
			}
			ModelRpcCmd::AirList { options, res_tx } => {
				let reply = match get_model_manager() {
					Ok(mm) => AirBmc::list(mm, Some(options)).await.map_err(map_model_err),
					Err(err) => Err(map_model_err(err)),
				};
				res_tx.send(ModelRpcReply::AirList(reply));
			}
			ModelRpcCmd::DbSize { res_tx } => {
				let reply = match get_model_manager() {
					Ok(mm) => mm.db_size().await.map_err(map_model_err),
					Err(err) => Err(map_model_err(err)),
				};
				res_tx.send(ModelRpcReply::DbSize(reply));
			}
		}
	}
}

fn map_model_err(err: impl std::fmt::Display) -> ModelRpcError {
	ModelRpcError::custom(err.to_string())
}

// endregion: --- Support
