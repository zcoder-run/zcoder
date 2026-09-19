// region:    --- ExecutorAction

use crate::model::Id;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecReq {
	pub wspace_id: Id,
	pub cmd: ExecCmd,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecCmd {
	RunPrompt(String),
}

pub type ExecReqRx = zc_common::event_base::MpscRx<ExecReq>;
pub type ExecReqTx = zc_common::event_base::MpscTx<ExecReq>;

pub type ExecCmdRx = ExecReqRx;
pub type ExecCmdTx = ExecReqTx;

// endregion: --- ExecutorAction

// region:    --- ExecStatus

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecEvent {
	RunStart(Id),
	RunEnd(Id),
	RunError(Id),
}

pub type ExecEventRx = zc_common::event_base::MpscRx<ExecEvent>;
pub type ExecEventTx = zc_common::event_base::MpscTx<ExecEvent>;

// endregion: --- ExecStatus
