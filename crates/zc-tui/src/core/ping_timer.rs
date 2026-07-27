use super::event::{PingTimerTx, TuiEvent, TuiTx};
use crate::Result;
use std::time::Duration;
use zc_common::event_base::new_mpsc_bounded;

#[allow(unused)]
pub fn start_ping_timer(tui_tx: TuiTx, tick_interval: Duration) -> Result<PingTimerTx> {
	let (stop_tx, mut stop_rx) = new_mpsc_bounded::<()>("ping_channel", 1000)?;

	tokio::spawn(async move {
		let mut interval = tokio::time::interval(tick_interval);

		loop {
			tokio::select! {
				_ = interval.tick() => {
					if tui_tx.send(TuiEvent::Tick).await.is_err() {
						break;
					}
				}
				_ = stop_rx.recv() => {
					break;
				}
			}
		}
	});

	Ok(stop_tx)
}
