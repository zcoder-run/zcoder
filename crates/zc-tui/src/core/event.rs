use crossterm::event::Event;
use zc_common::event_base::{MpscRx, MpscTx};
use zc_core::exec::ExecEvent;
use zc_core::model::ModelEvent;

// region:    --- Tui Event

pub type TuiTx = MpscTx<TuiEvent>;
pub type TuiRx = MpscRx<TuiEvent>;

#[derive(Debug, Clone)]
pub enum TuiEvent {
	Term(Event),
	Action(AppActionEvent),
	Exec(ExecEvent),
	Model(ModelEvent),
	Tick(i64),
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

pub type PingTimerTx = MpscTx<i64>;

// endregion: --- Ping Event
