mod cmd;
mod error;

use crate::cmd::CliCmd;
use clap::Parser as _;
pub use error::{Error, Result};
use zc_core::exec::{Executor, ExecutorConfig};

// -- Main
#[tokio::main]
async fn main() -> Result<()> {
	println!();

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
