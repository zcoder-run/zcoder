// region:    --- ExecutorAction

use crate::model::Id;

#[derive(Debug)]
pub enum ExecCmd {
	RunPrompt(String),
}

pub type ExecCmdRx = zc_common::event::MpscRx<ExecCmd>;
pub type ExecCmdTx = zc_common::event::MpscTx<ExecCmd>;

// endregion: --- ExecutorAction

// region:    --- ExecStatus

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum ExecEvent {
	RunStart(Id),
	RunEnd(Id),
	RunError(Id),
}

pub type ExecEventRx = zc_common::event::MpscRx<ExecEvent>;
pub type ExecEventTx = zc_common::event::MpscTx<ExecEvent>;

// endregion: --- ExecStatus
