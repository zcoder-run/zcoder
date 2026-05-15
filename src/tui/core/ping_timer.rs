use super::AppEvent;
use std::time::Duration;
use tokio::sync::mpsc::{self, Sender};

pub type PingTimerTx = Sender<()>;

pub fn start_ping_timer(app_tx: Sender<AppEvent>, tick_interval: Duration) -> PingTimerTx {
	let (stop_tx, mut stop_rx) = mpsc::channel(1);

	tokio::spawn(async move {
		let mut interval = tokio::time::interval(tick_interval);

		loop {
			tokio::select! {
				_ = interval.tick() => {
					if app_tx.send(AppEvent::Tick).await.is_err() {
						break;
					}
				}
				_ = stop_rx.recv() => {
					break;
				}
			}
		}
	});

	stop_tx
}
