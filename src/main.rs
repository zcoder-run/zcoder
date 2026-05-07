mod cmd;
mod error;

use crate::cmd::CliCmd;
use clap::Parser as _;
pub use error::{Error, Result};
use genai::chat::{ChatMessage, ChatRequest};
use inquire::Text;

// -- Consts
const MODEL: &str = "gemini-3-flash-preview";
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
	let mut current_prompt = cli_cmd.prompt;

	// -- Base chat request
	let chat_req = ChatRequest::from_system(format!(
		"You are a senior developer. User will give you instructions and context.\n\n{}",
		udiffx::prompt_file_changes()
	));

	loop {
		// -- Get Prompt
		let prompt = if let Some(p) = current_prompt.take() {
			p
		} else {
			match Text::new("Prompt >").prompt() {
				Ok(p) if !p.trim().is_empty() => p,
				_ => break,
			}
		};

		// clone base prompt
		let mut chat_req = chat_req.clone();
		// add user prompt
		chat_req = chat_req.append_message(ChatMessage::user(prompt));

		// -- Load files Context
		if let Some(files_context) = udiffx::load_files_context(&base_dir, SRC_GLOBS)? {
			chat_req = chat_req.append_message(ChatMessage::user(files_context));
		}

		// -- Execute Chat
		println!("... sending to {MODEL}");
		let res = client.exec_chat(MODEL, chat_req, None).await?;
		let ai_response = res.content.into_first_text().ok_or("Should have response")?;

		// -- Process AI Response
		let (file_changes, other_content) = udiffx::extract_file_changes(&ai_response, true)?;
		let _change_statuses = udiffx::apply_file_changes(&base_dir, file_changes)?;

		// -- Print ai answer
		let other_content = other_content.as_deref().unwrap_or("");
		println!("\nAI Answer:\n{}\n", other_content.trim());
	}

	Ok(())
}
