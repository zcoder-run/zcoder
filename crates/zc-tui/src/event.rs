use crossterm::event::Event;
use zc_common::event::{Rx, Tx};
use zc_core::exec::ExecEvent;
use zc_core::model::ModelEvent;

// region:    --- Tui Event

pub type TuiTx = Tx<TuiEvent>;
pub type TuiRx = Rx<TuiEvent>;

#[derive(Debug, Clone)]
pub enum TuiEvent {
	Term(Event),
	Action(AppActionEvent),
	Exec(ExecEvent),
	Model(ModelEvent),
	Tick,
	DoRedraw,
}

impl TuiEvent {
	/// Returns `true` if the tui event is [`Model`].
	///
	/// [`Model`]: TuiEvent::Model
	#[must_use]
	pub fn is_model(&self) -> bool {
		matches!(self, Self::Model(..))
	}
}

#[derive(Debug, Clone)]
pub enum AppActionEvent {
	Quit,
	RunPrompt(String),
}

// endregion: --- Tui Event

// region:    --- Ping Event

pub type PingTimerTx = Tx<()>;

// endregion: --- Ping Event
