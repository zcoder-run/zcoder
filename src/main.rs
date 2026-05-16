mod cmd;
mod error;
mod exec;
mod tui;

use crate::cmd::CliCmd;
use crate::exec::Executor;
use clap::Parser as _;
pub use error::{Error, Result};
use genai::chat::ChatRequest;
use tokio::sync::mpsc;

// -- Consts (harcoded for now)
const MODEL: &str = "gemini-3.1-flash-lite";

const SRC_GLOBS: &[&str] = &[
	// avoid root `**/..`
	"src/**/*.{py,ts,js,rs,html,css,json,toml}",
	"*.{py,ts,js,rs,html,css,json,toml,md}",
];

// -- Main
#[tokio::main]
async fn main() -> Result<()> {
	println!();

	// -- Cmd parsing & Setup
	let cli_cmd = CliCmd::parse();
	let base_dir = cli_cmd.dir.unwrap_or_else(|| ".demo-dir/".to_string());
	let client = genai::Client::default();

	// -- Base chat request
	let base_chat_req = ChatRequest::from_system(format!(
		"You are a senior developer. User will give you instructions and context.\n\n{}",
		udiffx::prompt_file_changes()
	));

	// -- Executor setup
	let (status_tx, status_rx) = mpsc::channel(100);
	let (executor, executor_tx) = Executor::new(status_tx, client, base_chat_req, base_dir, MODEL, SRC_GLOBS);

	tokio::spawn(async move {
		executor.start().await;
	});

	// -- Running Tui application
	crate::tui::start_tui(executor_tx, status_rx, cli_cmd.prompt).await?;

	Ok(())
}
