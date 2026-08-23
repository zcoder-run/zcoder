use super::event::{PingTimerTx, TuiEvent, TuiTx};
use crate::Result;
use std::pin::Pin;
use std::time::Duration;
use tokio::time::Sleep;
use zc_common::event_base::new_mpsc_bounded;

pub fn start_ping_timer(tui_tx: TuiTx) -> Result<PingTimerTx> {
	let (tx, mut rx) = new_mpsc_bounded::<i64>("ping_timer", 1000)?;

	tokio::spawn(async move {
		let mut pending_ts: Option<i64> = None;
		let mut sleep_fut: Option<Pin<Box<Sleep>>> = None;

		loop {
			if let Some(sleep) = sleep_fut.as_mut() {
				tokio::select! {
					_ = sleep.as_mut() => {
						if let Some(ts) = pending_ts.take() {
							let _ = tui_tx.send(TuiEvent::Tick(ts)).await;
						}
						sleep_fut = None;
					}
					msg = rx.recv() => {
						match msg {
							Ok(ts) => {
								pending_ts = Some(ts);
							}
							Err(_) => break,
						}
					}
				}
			} else {
				match rx.recv().await {
					Ok(ts) => {
						pending_ts = Some(ts);
						sleep_fut = Some(Box::pin(tokio::time::sleep(Duration::from_millis(100))));
					}
					Err(_) => break,
				}
			}
		}
	});

	Ok(tx)
}
