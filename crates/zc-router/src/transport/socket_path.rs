// region:    --- Modules

use crate::error::Result;
use simple_fs::SPath;
use std::path::Path;
use tokio::net::UnixStream;

// endregion: --- Modules

// region:    --- Socket Path Policy

/// Returns the shared base socket path as an [`SPath`].
pub fn socket_path() -> SPath {
	SPath::from(zc_common::consts::BASE_SOCK_PATH)
}

/// Removes the socket file, ignoring a missing path.
pub fn unlink_if_exists(socket_path: impl AsRef<Path>) -> Result<()> {
	match std::fs::remove_file(socket_path.as_ref()) {
		Ok(()) => Ok(()),
		Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
		Err(err) => Err(err.into()),
	}
}

/// Returns `true` only when a server answers a connect on the path.
pub async fn is_live(socket_path: impl AsRef<Path>) -> bool {
	UnixStream::connect(socket_path.as_ref()).await.is_ok()
}

// endregion: --- Socket Path Policy

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[test]
	fn test_socket_path_matches_const() -> Result<()> {
		// -- Exec
		let path = socket_path();

		// -- Check
		assert_eq!(path.as_str(), zc_common::consts::BASE_SOCK_PATH);

		Ok(())
	}

	#[test]
	fn test_socket_path_unlink_if_exists_missing() -> Result<()> {
		// -- Setup & Fixtures
		let path = Path::new("/tmp/zcoder-test-unlink-missing.sock");

		// -- Exec
		unlink_if_exists(path)?;

		// -- Check
		assert!(!path.exists());

		Ok(())
	}

	#[tokio::test]
	async fn test_socket_path_is_live_not_owned() -> Result<()> {
		// -- Setup & Fixtures
		let path = Path::new("/tmp/zcoder-test-live-not-owned.sock");

		// -- Exec
		let live = is_live(path).await;

		// -- Check
		assert!(!live);

		Ok(())
	}
}

// endregion: --- Tests
