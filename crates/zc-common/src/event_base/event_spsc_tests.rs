type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>; // For tests.

use super::*;

#[tokio::test]
async fn test_event_base_spsc_send_recv() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, mut rx) = new_spsc_bounded::<u32>("spsc-send-recv", 2)?;

	// -- Exec
	tx.send(7).await?;
	let value = rx.recv().await?;

	// -- Check
	assert_eq!(value, 7);
	assert_eq!(tx.name(), "spsc-send-recv");
	assert_eq!(rx.name(), "spsc-send-recv");
	Ok(())
}

#[tokio::test]
async fn test_event_base_spsc_bounded_default() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, mut rx) = new_spsc_bounded_default::<u32>("spsc-default")?;

	// -- Exec
	tx.send(11).await?;
	let value = rx.recv().await?;

	// -- Check
	assert_eq!(value, 11);
	Ok(())
}

#[test]
fn test_event_base_spsc_invalid_capacity() -> Result<()> {
	// -- Setup & Fixtures
	let name = "spsc-invalid-capacity";

	// -- Exec
	let result = new_spsc_bounded::<u32>(name, 0);

	// -- Check
	assert!(matches!(
		result,
		Err(EventBaseError::InvalidCapacity {
			name: "spsc-invalid-capacity",
			capacity: 0
		})
	));
	Ok(())
}

#[test]
fn test_event_base_spsc_try_operations() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, rx) = new_spsc_bounded::<u32>("spsc-try", 1)?;

	// -- Exec
	let first = tx.try_send(1)?;
	let second = tx.try_send(2)?;
	let received = rx.try_recv()?;
	let empty = rx.try_recv()?;

	// -- Check
	assert!(first.is_none());
	assert_eq!(second, Some(2));
	assert_eq!(received, Some(1));
	assert_eq!(empty, None);
	Ok(())
}

#[tokio::test]
async fn test_event_base_spsc_sender_disconnected() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, mut rx) = new_spsc_bounded::<u32>("spsc-tx-disconnected", 1)?;
	drop(tx);

	// -- Exec
	let result = rx.recv().await;

	// -- Check
	assert!(matches!(
		result,
		Err(EventBaseError::RxDisconnected {
			name: "spsc-tx-disconnected"
		})
	));
	Ok(())
}

#[tokio::test]
async fn test_event_base_spsc_receiver_disconnected() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, rx) = new_spsc_bounded::<u32>("spsc-rx-disconnected", 1)?;
	drop(rx);

	// -- Exec
	let result = tx.send(1).await;

	// -- Check
	assert!(matches!(
		result,
		Err(EventBaseError::TxDisconnected {
			name: "spsc-rx-disconnected"
		})
	));
	Ok(())
}

#[test]
fn test_event_base_spsc_send_sync() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, rx) = new_spsc_bounded::<u32>("spsc-send-sync", 1)?;

	// -- Exec
	tx.send_sync(5)?;
	let value = rx.try_recv()?;

	// -- Check
	assert_eq!(value, Some(5));
	Ok(())
}

#[test]
fn test_event_base_spsc_send_sync_disconnected() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, rx) = new_spsc_bounded::<u32>("spsc-send-sync-disconnected", 1)?;
	drop(rx);

	// -- Exec
	let result = tx.send_sync(5);

	// -- Check
	assert!(matches!(
		result,
		Err(EventBaseError::TxDisconnected {
			name: "spsc-send-sync-disconnected"
		})
	));
	Ok(())
}

#[test]
fn test_event_base_spsc_send_sync_waits_for_capacity() -> Result<()> {
	// -- Setup & Fixtures
	let (tx, rx) = new_spsc_bounded::<u32>("spsc-send-sync-blocking", 1)?;
	let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);

	tx.send_sync(1)?;
	let sender = std::thread::spawn(move || tx.send_sync(2));

	// -- Exec
	// Let the sender thread reach the full channel and park before capacity is freed.
	std::thread::sleep(std::time::Duration::from_millis(50));

	let mut received = vec![];
	while received.len() < 2 && std::time::Instant::now() < deadline {
		match rx.try_recv()? {
			Some(value) => received.push(value),
			None => std::thread::sleep(std::time::Duration::from_millis(1)),
		}
	}

	while !sender.is_finished() && std::time::Instant::now() < deadline {
		std::thread::sleep(std::time::Duration::from_millis(1));
	}

	// -- Check
	assert_eq!(received, vec![1, 2]);
	assert!(
		sender.is_finished(),
		"send_sync must complete once capacity is available"
	);
	sender.join().map_err(|_e| "sender thread panicked")??;
	Ok(())
}
