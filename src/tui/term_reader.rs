use crate::tui::AppEvent;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use tokio::sync::mpsc::Sender;

pub fn run_term_reader(app_tx: Sender<AppEvent>) {
	tokio::spawn(async move {
		let mut reader = EventStream::new();

		while let Some(maybe_event) = reader.next().await {
			match maybe_event {
				Ok(evt) => {
					// Only process key press events (ignore repeats and releases)
					if let Event::Key(key) = &evt
						&& key.kind != KeyEventKind::Press
					{
						continue;
					}

					if app_tx.send(AppEvent::Term(evt)).await.is_err() {
						break;
					}
				}
				Err(_) => break,
			}
		}
	});
}
