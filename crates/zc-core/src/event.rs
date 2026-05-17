#[derive(Debug)]
pub enum ExecActionEvent {
	RunPrompt(String),
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone)]
pub enum ExecStatusEvent {
	RunStart,
	RunEnd,
	RunResult(String),
	RunError(String),
}
