// region:    --- ExecutorAction

#[derive(Debug)]
pub enum ExecAction {
	RunPrompt(String),
}

pub type ExecActionRx = zc_common::event::Rx<ExecAction>;
pub type ExecActionTx = zc_common::event::Tx<ExecAction>;

// endregion: --- ExecutorAction

// region:    --- ExecStatus

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum ExecEvent {
	RunStart,
	RunEnd,
	RunResult(String),
	RunError(String),
}

pub type ExecutorStatusRx = zc_common::event::Rx<ExecEvent>;
pub type ExecutorStatusTx = zc_common::event::Tx<ExecEvent>;

// endregion: --- ExecStatus
