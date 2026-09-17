// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub mod exec;
pub mod exec_event;
pub mod model_change;
pub mod model_rpc;
pub mod msg;
pub mod router;

pub use exec::ExecCmd;
pub use exec_event::{ExecEvent, new_exec_event_channel};
pub use model_change::{ModelChangeEvent, ModelChangeRx, ModelChangeTx, new_model_change_channel};
pub use model_rpc::{
	ModelRpcCmd, ModelRpcCmdRx, ModelRpcCmdTx, ModelRpcError, ModelRpcReply, ModelRpcResult, air_get, air_list,
	db_size, new_model_rpc_cmd_channel, run_get, run_list,
};
pub use msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx, new_router_msg_channel};
pub use router::{route, run_router};

// endregion: --- Modules
