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
pub use exec_event::{ExecEvent, new_exec_event_channel, run_exec_event_loop};
pub use model_change::{
	ModelChangeEvent, ModelChangeRx, ModelChangeTx, new_model_change_channel, run_model_change_loop,
};
pub use model_rpc::{ModelRpcCmd, ModelRpcError, ModelRpcReply, ModelRpcResult, air_get, air_list, run_get, run_list};
pub use msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx, new_router_msg_channel};
pub use router::{route, run_router};

// endregion: --- Modules
