use crossterm::event::Event;
use zc_common::event::{Mp, Sc};
use zc_core::exec::ExecEvent;
use zc_core::model::ModelEvent;

// region:    --- Tui Event

pub type TuiTx = Mp<TuiEvent>;
pub type TuiRx = Sc<TuiEvent>;

#[derive(Debug, Clone)]
pub enum TuiEvent {
	Term(Event),
	Action(AppActionEvent),
	Exec(ExecEvent),
	Model(ModelEvent),
	Tick,
	#[allow(unused)]
	DoRedraw,
}

#[derive(Debug, Clone)]
pub enum AppActionEvent {
	Quit,
	RunPrompt(String),
}

// endregion: --- Tui Event

// region:    --- Ping Event

pub type PingTimerTx = Mp<()>;

// endregion: --- Ping Event
