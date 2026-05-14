mod executor;

pub use executor::*;

#[derive(Debug)]
pub enum ExecActionEvent {
	RunPrompt(String),
}

#[derive(Debug, Clone)]
pub enum ExecStatusEvent {
	RunStart,
	RunEnd,
	RunResult(String),
	RunError(String),
}
