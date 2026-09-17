use crate::model::{AirBmc, ModelManager, RunBmc, get_model_manager};
use zc_router::{ModelRpcCmd, ModelRpcCmdRx, ModelRpcError, ModelRpcReply, ModelRpcResult};

// region:    --- Model RPC Handler

/// Runs the model RPC handler loop, serving the read requests that the router forwards
/// by reading Core state.
pub async fn run_model_rpc_handler(mut model_rpc_cmd_rx: ModelRpcCmdRx) {
	while let Ok(cmd) = model_rpc_cmd_rx.recv().await {
		handle_model_rpc_cmd(cmd).await;
	}
}

// endregion: --- Model RPC Handler

// region:    --- Support

async fn handle_model_rpc_cmd(cmd: ModelRpcCmd) {
	tracing::debug!("->> handling model rpc cmd {cmd:?}");

	match cmd {
		ModelRpcCmd::RunGet { id, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => RunBmc::get(mm, id).await.map(Some).map_err(map_model_err),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::Run(reply));
		}
		ModelRpcCmd::RunList { options, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => RunBmc::list(mm, Some(options)).await.map_err(map_model_err),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::RunList(reply));
		}
		ModelRpcCmd::AirGet { id, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => AirBmc::get(mm, id).await.map(Some).map_err(map_model_err),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::Air(reply));
		}
		ModelRpcCmd::AirList { options, res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => AirBmc::list(mm, Some(options)).await.map_err(map_model_err),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::AirList(reply));
		}
		ModelRpcCmd::DbSize { res_tx } => {
			let reply = match model_manager() {
				Ok(mm) => mm.db_size().await.map_err(map_model_err),
				Err(err) => Err(err),
			};
			res_tx.send(ModelRpcReply::DbSize(reply));
		}
	}
}

fn model_manager() -> ModelRpcResult<&'static ModelManager> {
	get_model_manager().map_err(ModelRpcError::custom)
}

fn map_model_err(err: crate::model::Error) -> ModelRpcError {
	ModelRpcError::custom(err.to_string())
}

// endregion: --- Support
