// region:    --- Modules

use crate::{Error, Result};
use std::path::Path;
use zc_router::{ClientInfo, RouterClient};

// endregion: --- Modules

// region:    --- Detached Base Spawner

/// Spawns the `zc base` daemon as a fully detached background process.
///
/// On Unix, configures a separate process group (`process_group(0)`) so the child
/// process does not receive signals (such as SIGINT / Ctrl-C or SIGHUP) delivered
/// to the parent's terminal session. Stdio is redirected to null.
pub async fn spawn_base_detached() -> Result<()> {
	let current_exe = std::env::current_exe()
		.map_err(|err| Error::custom(format!("failed to get current executable path: {err}")))?;

	let mut cmd = std::process::Command::new(current_exe);
	cmd.arg("base")
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null());

	#[cfg(unix)]
	{
		use std::os::unix::process::CommandExt;
		cmd.process_group(0);
	}

	cmd.spawn()
		.map_err(|err| Error::custom(format!("failed to spawn zc base: {err}")))?;

	Ok(())
}

/// Connects to a running base daemon or spawns a new fully detached base instance.
pub async fn connect_or_spawn(socket_path: impl AsRef<Path>, client_info: ClientInfo) -> Result<RouterClient> {
	let socket_path = socket_path.as_ref();
	match RouterClient::uds(socket_path, client_info.clone()).await {
		Ok(client) => return Ok(client),
		Err(err) => {
			if zc_router::transport::is_live(socket_path).await {
				return Err(err.into());
			}
		}
	}

	spawn_base_detached().await?;

	let start = std::time::Instant::now();
	let timeout = std::time::Duration::from_secs(5);
	let mut delay = std::time::Duration::from_millis(50);

	while start.elapsed() < timeout {
		tokio::time::sleep(delay).await;
		match RouterClient::uds(socket_path, client_info.clone()).await {
			Ok(client) => return Ok(client),
			Err(err) => {
				if zc_router::transport::is_live(socket_path).await {
					return Err(err.into());
				}
				delay = (delay * 2).min(std::time::Duration::from_millis(250));
			}
		}
	}

	Err(Error::custom(format!(
		"could not connect to zc base at '{}' within timeout",
		socket_path.display()
	)))
}

// endregion: --- Detached Base Spawner

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[test]
	fn test_base_spawner_exe_resolves() -> Result<()> {
		let current_exe = std::env::current_exe()?;
		assert!(current_exe.is_file());
		Ok(())
	}
}

// endregion: --- Tests
