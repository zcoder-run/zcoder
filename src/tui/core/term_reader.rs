use super::AppEvent;
use crossterm::event::EventStream;
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use tokio::sync::mpsc::Sender;

pub fn run_term_reader(app_tx: Sender<AppEvent>) {
	tokio::spawn(async move {
		let mut reader = EventStream::new();

		loop {
			let delay = tokio::time::sleep(Duration::from_millis(200)).fuse();
			let event = reader.next().fuse();

			futures::pin_mut!(delay);
			futures::pin_mut!(event);

			tokio::select! {
				_ = &mut delay => {}
				maybe_event = &mut event => {
					match maybe_event {
						Some(Ok(evt)) => {
							if app_tx.send(AppEvent::Term(evt)).await.is_err() {
								break;
							}
						}
						Some(Err(_)) => {}
						None => break,
					}
				}
			}
		}
	});
}
