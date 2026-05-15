use crate::exec::ExecStatusEvent;
use crossterm::event::Event;

#[derive(Debug)]
pub enum AppEvent {
	Term(Event),
	Action(AppActionEvent),
	Exec(ExecStatusEvent),
	Tick,
	DoRedraw,
}

#[derive(Debug)]
pub enum AppActionEvent {
	Quit,
	RunPrompt(String),
}
