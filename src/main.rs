mod cmd;
mod error;

use crate::cmd::CliCmd;
use clap::Parser as _;
pub use error::{Error, Result};
use tracing_appender::rolling::never;
use tracing_subscriber::EnvFilter;
use zc_core::exec::{Executor, ExecutorConfig};

const DEBUG_LOG: bool = true;

// -- Main
#[tokio::main]
async fn main() -> Result<()> {
	println!();

	// -- Setup debug tracing_subscriber
	// NOTE: need to keep the handle, otherwise dropped, and nothing get added to the file
	let _tracing_guard = if DEBUG_LOG {
		// Create a file appender (will write all logs to ".tmp.log" in the current dir)
		let file_appender = never(".zcoder/debug-log", "log.txt");
		let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

		// Set up the subscriber with the file writer and log level
		tracing_subscriber::fmt()
			.with_writer(non_blocking)
			.with_env_filter(EnvFilter::new("zcoder=debug,zc_tui=debug,zc_core=debug"))
			.without_time()
			.with_ansi(false)
			.init();
		// }
		Some(_guard)
	} else {
		None
	};

	// -- Cmd parsing & Setup
	let cli_cmd = CliCmd::parse();
	let base_dir = cli_cmd.dir.unwrap_or_else(|| ".demo-dir/".to_string());

	// -- Executor setup
	let executor_config = ExecutorConfig::default().with_base_dir(base_dir);
	let (executor, executor_tx, status_rx) = Executor::new(executor_config);

	tokio::spawn(async move { executor.start().await });

	// -- Running Tui application
	zc_tui::start_tui(executor_tx, status_rx, cli_cmd.prompt).await?;

	Ok(())
}
