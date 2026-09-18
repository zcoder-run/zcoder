mod base_cmd;
mod cmd;
mod error;

use std::path::Path;

use crate::cmd::{CliCmd, SubCmd};
use clap::Parser as _;
pub use error::{Error, Result};
use simple_fs::SPath;
use tracing_appender::rolling::never;
use tracing_subscriber::EnvFilter;
use zc_common::dirs::{find_wks_dir, wks_log_file};
use zc_router::transport::socket_path;
use zc_router::{ClientInfo, RouterClient};

const DEBUG_LOG: bool = true;

// -- Main
#[tokio::main]
async fn main() -> Result<()> {
	// -- Cmd parsing & Setup
	let cli_cmd = CliCmd::parse();

	if cli_cmd.base || matches!(cli_cmd.command, Some(SubCmd::Base)) {
		return base_cmd::run_base_cmd().await;
	}

	let from_dir = if let Some(dir) = cli_cmd.dir.as_deref() {
		SPath::from(dir)
	} else {
		simple_fs::current_dir()?
	};
	let wks_dir = find_wks_dir(&from_dir).unwrap_or(from_dir);

	// -- Setup debug tracing_subscriber
	// NOTE: need to keep the handle, otherwise dropped, and nothing get added to the file
	let _tracing_guard = if DEBUG_LOG {
		let log_file = wks_log_file(&wks_dir);
		let log_dir = log_file.parent().unwrap_or_else(|| wks_dir.clone());
		let file_name = log_file.file_name().unwrap_or(zc_common::consts::DEBUG_LOG_FILE_NAME);
		let file_appender = never(log_dir.as_str(), file_name);
		let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

		// Set up the subscriber with the file writer and log level
		tracing_subscriber::fmt()
			.with_writer(non_blocking)
			.with_env_filter(EnvFilter::new(
				"zcoder=debug,zc_tui=debug,zc_core=debug,zc_base=debug,aicost=debug",
			))
			.without_time()
			.with_ansi(false)
			.init();
		Some(_guard)
	} else {
		None
	};

	println!();

	let sock_path = socket_path();
	let client_info = ClientInfo::from_wks_dir(wks_dir.as_str());
	let client = connect_or_spawn(&sock_path, client_info).await?;

	// -- Running Tui application
	zc_tui::start_tui(client, cli_cmd.prompt).await?;

	Ok(())
}

// region:    --- Support

async fn connect_or_spawn(socket_path: impl AsRef<Path>, client_info: ClientInfo) -> Result<RouterClient> {
	let socket_path = socket_path.as_ref();
	match RouterClient::uds(socket_path, client_info.clone()).await {
		Ok(client) => return Ok(client),
		Err(err) => {
			if zc_router::transport::is_live(socket_path).await {
				return Err(err.into());
			}
		}
	}

	let current_exe = std::env::current_exe()
		.map_err(|err| Error::custom(format!("failed to get current executable path: {err}")))?;

	std::process::Command::new(current_exe)
		.arg("base")
		.stdin(std::process::Stdio::null())
		.stdout(std::process::Stdio::null())
		.stderr(std::process::Stdio::null())
		.spawn()
		.map_err(|err| Error::custom(format!("failed to spawn zc base: {err}")))?;

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

// endregion: --- Support
