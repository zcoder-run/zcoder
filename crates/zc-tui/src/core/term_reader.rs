use super::event::TuiEvent;
use crossterm::event::EventStream;
use futures::{FutureExt, StreamExt};
use std::time::Duration;
use zc_common::event::MpscTx;

pub fn run_term_reader(tui_tx: MpscTx<TuiEvent>) {
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
							if tui_tx.send(TuiEvent::Term(evt)).await.is_err() {
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
