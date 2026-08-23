use crate::core::event::TuiEvent;
use std::collections::HashMap;
use zc_common::event_base::MpscRx;
use zc_core::model::{EntityType, Id};

// region:    --- Debouncer

pub struct Debouncer {
	last_redraw_event: Option<TuiEvent>,
	ui_events: Vec<TuiEvent>,
	exec_events: Vec<TuiEvent>,
	event_by_key: HashMap<(EntityType, Option<Id>), TuiEvent>,
	tick_event: Option<TuiEvent>,
}

impl Debouncer {
	pub fn new(first_event: TuiEvent) -> Self {
		let mut debouncer = Self {
			last_redraw_event: None,
			ui_events: Vec::new(),
			exec_events: Vec::new(),
			event_by_key: HashMap::new(),
			tick_event: None,
		};
		debouncer.process(first_event);
		debouncer
	}

	pub fn process(&mut self, app_event: TuiEvent) {
		match app_event {
			TuiEvent::DoRedraw => {
				self.last_redraw_event = Some(TuiEvent::DoRedraw);
			}
			TuiEvent::Term(event) => {
				self.ui_events.push(TuiEvent::Term(event));
			}
			TuiEvent::Action(action_event) => {
				self.ui_events.push(TuiEvent::Action(action_event));
			}
			TuiEvent::Exec(exec_event) => {
				self.exec_events.push(TuiEvent::Exec(exec_event));
			}
			TuiEvent::Model(model_event) => {
				let key = (model_event.entity, model_event.id);
				self.event_by_key.insert(key, TuiEvent::Model(model_event));
			}
			TuiEvent::Tick(ts) => {
				self.tick_event = Some(TuiEvent::Tick(ts));
			}
		}
	}

	pub fn into_events(self) -> Vec<TuiEvent> {
		let mut events = self.ui_events;
		events.extend(self.exec_events);
		events.extend(self.event_by_key.into_values());
		if let Some(last_redraw_event) = self.last_redraw_event {
			events.push(last_redraw_event);
		}
		if let Some(tick) = self.tick_event {
			events.push(tick);
		}
		events
	}
}

pub fn debounce_events(tui_rx: &mut MpscRx<TuiEvent>, first_event: TuiEvent) -> Vec<TuiEvent> {
	let mut debouncer = Debouncer::new(first_event);
	loop {
		match tui_rx.try_recv() {
			Ok(Some(app_event)) => {
				debouncer.process(app_event);
			}
			Ok(None) => break,
			Err(_) => break,
		}
	}
	debouncer.into_events()
}

// endregion: --- Debouncer

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use crate::core::event::AppActionEvent;
	use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
	use zc_common::event_base::new_mpsc_bounded;
	use zc_core::exec::ExecEvent;
	use zc_core::model::{EntityAction, ModelEvent, RelIds};

	#[test]
	fn test_debouncer_tick_coalescing() -> Result<()> {
		// -- Setup & Fixtures
		let mut debouncer = Debouncer::new(TuiEvent::Tick(100));
		debouncer.process(TuiEvent::Tick(200));
		debouncer.process(TuiEvent::Tick(300));
		debouncer.process(TuiEvent::Tick(400));

		// -- Exec
		let events = debouncer.into_events();

		// -- Check: only the newest tick timestamp is retained
		assert_eq!(events.len(), 1);
		match events.first().ok_or("should have 1 event")? {
			TuiEvent::Tick(ts) => assert_eq!(*ts, 400),
			_ => return Err("expected TuiEvent::Tick".into()),
		}

		Ok(())
	}

	#[test]
	fn test_debouncer_fifo_ui_events() -> Result<()> {
		// -- Setup & Fixtures
		let key1 = TuiEvent::Term(crossterm::event::Event::Key(KeyEvent {
			code: KeyCode::Char('a'),
			modifiers: KeyModifiers::empty(),
			kind: KeyEventKind::Press,
			state: KeyEventState::empty(),
		}));
		let key2 = TuiEvent::Term(crossterm::event::Event::Key(KeyEvent {
			code: KeyCode::Char('b'),
			modifiers: KeyModifiers::empty(),
			kind: KeyEventKind::Press,
			state: KeyEventState::empty(),
		}));
		let action1 = TuiEvent::Action(AppActionEvent::RunPrompt("test".to_string()));

		let mut debouncer = Debouncer::new(key1);
		debouncer.process(key2);
		debouncer.process(action1);

		// -- Exec
		let events = debouncer.into_events();

		// -- Check: exact FIFO preserved for UI events
		assert_eq!(events.len(), 3);
		assert!(matches!(events[0], TuiEvent::Term(_)));
		assert!(matches!(events[1], TuiEvent::Term(_)));
		assert!(matches!(events[2], TuiEvent::Action(_)));

		Ok(())
	}

	#[test]
	fn test_debouncer_redraw_and_model_deduplication() -> Result<()> {
		// -- Setup & Fixtures
		let test_id = Id::default();
		let model1 = TuiEvent::Model(ModelEvent::new(
			EntityType::Run,
			EntityAction::Updated,
			Some(test_id),
			RelIds::default(),
		));
		let model2 = TuiEvent::Model(ModelEvent::new(
			EntityType::Run,
			EntityAction::Updated,
			Some(test_id),
			RelIds::default(),
		));

		let mut debouncer = Debouncer::new(TuiEvent::DoRedraw);
		debouncer.process(TuiEvent::DoRedraw);
		debouncer.process(model1);
		debouncer.process(model2);
		debouncer.process(TuiEvent::DoRedraw);

		// -- Exec
		let events = debouncer.into_events();

		// -- Check: 1 model event for Run(42) and 1 DoRedraw
		assert_eq!(events.len(), 2);
		let model_count = events.iter().filter(|e| matches!(e, TuiEvent::Model(_))).count();
		let redraw_count = events.iter().filter(|e| matches!(e, TuiEvent::DoRedraw)).count();
		assert_eq!(model_count, 1);
		assert_eq!(redraw_count, 1);

		Ok(())
	}

	#[tokio::test]
	async fn test_debounce_events_channel_drain() -> Result<()> {
		// -- Setup & Fixtures
		let (tx, mut rx) = new_mpsc_bounded::<TuiEvent>("test_debounce", 10)?;
		let run_id = Id::default();
		tx.send(TuiEvent::Tick(100)).await?;
		tx.send(TuiEvent::Exec(ExecEvent::RunStart(run_id))).await?;
		tx.send(TuiEvent::Tick(250)).await?;

		// -- Exec
		let first_event = TuiEvent::DoRedraw;
		let events = debounce_events(&mut rx, first_event);

		// -- Check: drained channel, coalesced ticks, preserved Exec and Redraw
		assert_eq!(events.len(), 3);
		let has_exec = events.iter().any(|e| matches!(e, TuiEvent::Exec(_)));
		let has_redraw = events.iter().any(|e| matches!(e, TuiEvent::DoRedraw));
		let tick = events.iter().find_map(|e| match e {
			TuiEvent::Tick(ts) => Some(*ts),
			_ => None,
		});

		assert!(has_exec);
		assert!(has_redraw);
		assert_eq!(tick, Some(250));

		Ok(())
	}
}

// endregion: --- Tests
