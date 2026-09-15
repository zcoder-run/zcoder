// region:    --- Modules

mod error;

pub use error::{Error, Result};

pub mod exec;
pub mod model_rpc;
pub mod msg;
pub mod router;

pub use exec::ExecCmd;
pub use model_rpc::ModelRpcCmd;
pub use msg::{CoreMsg, CoreMsgData, CoreMsgRx, CoreMsgTx, new_core_msg_channel};
pub use router::{route, run_router};

// endregion: --- Modules
