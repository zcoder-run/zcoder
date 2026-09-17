// region:    --- Modules

use zc_base::model::{AirBmc, RunBmc, get_model_manager};
use zc_common::event_base::new_mpsc_bounded;
use zc_router::{
	ModelRpcCmd, ModelRpcCmdRx, ModelRpcError, ModelRpcReply, RouterClient, new_model_rpc_cmd_channel, run_router,
};

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

// endregion: --- Modules

// region:    --- Test Router Support

/// Starts a router wired to a local model RPC stub for the `zc-tui` tests.
///
/// `zc-tui` does not own the model layer, so the router forwards its model RPC
/// commands to the stub below, which serves them by reading the model manager.
/// This mirrors what `zc-base` does in production and keeps the tests runnable
/// without a base process.
pub fn start_router_with_stub() -> Result<RouterClient> {
	let (router_msg_tx, router_msg_rx) = new_mpsc_bounded("test_router_msg", 10)?;
	let (reply_tx, reply_rx) = new_mpsc_bounded("test_router_reply", 10)?;
	let (exec_cmd_tx, _exec_cmd_rx) = new_mpsc_bounded("test_exec_cmd", 10)?;
	let (_model_change_tx, model_change_rx) = new_mpsc_bounded("test_model_change", 10)?;
	let (_exec_event_tx, exec_event_rx) = new_mpsc_bounded("test_exec_event", 10)?;

	let (model_rpc_cmd_tx, model_rpc_cmd_rx) = new_model_rpc_cmd_channel();
	tokio::spawn(run_model_rpc_stub(model_rpc_cmd_rx));
	tokio::spawn(run_router(router_msg_rx, exec_cmd_tx, model_rpc_cmd_tx, reply_tx));

	Ok(RouterClient::in_proc(
		router_msg_tx,
		model_change_rx,
		exec_event_rx,
		reply_rx,
	))
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
