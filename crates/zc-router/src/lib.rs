// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub mod client;
pub mod client_filter;
pub mod client_info;
pub mod exec;
pub mod exec_event;
pub mod model_change;
pub mod model_rpc;
pub mod msg;
pub mod router;
#[cfg(feature = "server")]
pub mod server;
pub mod transport;
pub mod wspace_resolver;

pub use client::RouterClient;
pub use client_filter::client_filter;
pub use client_info::ClientInfo;
pub use exec::{ExecCmd, ExecCmdRx, ExecCmdTx, ExecReq, ExecReqRx, ExecReqTx};
pub use exec_event::{ExecEvent, ExecEventRx, ExecEventTx, new_exec_event_channel};
pub use model_change::{ModelChangeEvent, ModelChangeRx, ModelChangeTx, new_model_change_channel};
pub use model_rpc::{
	ModelRpcCmd, ModelRpcCmdRx, ModelRpcCmdTx, ModelRpcError, ModelRpcReply, ModelRpcReq, ModelRpcResult, air_get,
	air_list, db_size, new_model_rpc_cmd_channel, run_get, run_list,
};
pub use msg::{RouterMsg, RouterMsgData, RouterMsgRx, RouterMsgTx, new_router_msg_channel};
pub use router::{route, run_router};
#[cfg(feature = "server")]
pub use server::{ConnWatch, RouterServer};
pub use wspace_resolver::WksResolver;

// endregion: --- Modules
