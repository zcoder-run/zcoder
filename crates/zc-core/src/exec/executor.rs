use crate::exec::{
	Error, ExecAction, ExecActionRx, ExecActionTx, ExecEvent, ExecutorStatusRx, ExecutorStatusTx, Result,
};
use genai::chat::{ChatMessage, ChatRequest};
use zc_common::event::new_mpsc_bounded;

// -- Consts (harcoded for now)
const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

const DEFAULT_SRC_GLOBS: &[&str] = &[
	// avoid root `**/..`
	"src/**/*.{py,ts,js,rs,html,css,json,toml}",
	"*.{py,ts,js,rs,html,css,json,toml,md}",
];

pub struct Executor {
	action_rx: ExecActionRx,
	inner: ExecutorInner,
}

struct ExecutorInner {
	status_tx: ExecutorStatusTx,
	// State needed for execution
	genai_client: genai::Client,
	base_chat_req: ChatRequest,
	base_dir: String,
	model: &'static str,
	src_globs: &'static [&'static str],
}

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
	base_dir: String,
	model: &'static str,
	src_globs: &'static [&'static str],
}

impl Default for ExecutorConfig {
	fn default() -> Self {
		Self {
			base_dir: ".demo-dir/".to_string(),
			model: DEFAULT_MODEL,
			src_globs: DEFAULT_SRC_GLOBS,
		}
	}
}

impl ExecutorConfig {
	pub fn with_base_dir(mut self, base_dir: impl Into<String>) -> Self {
		self.base_dir = base_dir.into();
		self
	}

	pub fn with_model(mut self, model: &'static str) -> Self {
		self.model = model;
		self
	}

	pub fn with_src_globs(mut self, src_globs: &'static [&'static str]) -> Self {
		self.src_globs = src_globs;
		self
	}
}

impl Executor {
	pub fn new(config: ExecutorConfig) -> (Self, ExecActionTx, ExecutorStatusRx) {
		let (action_tx, action_rx) = new_mpsc_bounded::<ExecAction>();
		let (status_tx, status_rx) = new_mpsc_bounded::<ExecEvent>();

		let base_chat_req = ChatRequest::from_system(format!(
			"You are a senior developer. User will give you instructions and context.\n\n{}",
			udiffx::prompt_file_changes()
		));

		(
			Self {
				action_rx,
				inner: ExecutorInner {
					status_tx,
					genai_client: genai::Client::default(),
					base_chat_req,
					base_dir: config.base_dir,
					model: config.model,
					src_globs: config.src_globs,
				},
			},
			action_tx,
			status_rx,
		)
	}

	pub async fn start(self) -> Result<()> {
		let Self { mut action_rx, inner } = self;

		while let Ok(action) = action_rx.recv().await {
			match action {
				ExecAction::RunPrompt(prompt) => {
					let _ = inner.handle_run_prompt(prompt).await;
				}
			}
		}

		Ok(())
	}
}

impl ExecutorInner {
	async fn handle_run_prompt(&self, prompt: String) -> Result<()> {
		let _ = self.status_tx.send(ExecEvent::RunStart).await;

		let mut chat_req = self.base_chat_req.clone();
		chat_req = chat_req.append_message(ChatMessage::user(prompt));

		// -- Load files Context
		let files_context = udiffx::load_files_context(&self.base_dir, self.src_globs)?;
		if let Some(files_context) = files_context {
			chat_req = chat_req.append_message(ChatMessage::user(files_context));
		}

		// -- Execute Chat
		let res = self.genai_client.exec_chat(self.model, chat_req, None).await;

		let result = match res {
			Ok(res) => {
				let ai_response = res
					.content
					.into_first_text()
					.ok_or_else(|| Error::custom("Should have response"))?;

				// -- Process AI Response
				let (file_changes, other_content) = udiffx::extract_file_changes(&ai_response, true)?;
				let _change_statuses = udiffx::apply_file_changes(&self.base_dir, file_changes)?;

				Ok(other_content.unwrap_or_default())
			}
			Err(err) => Err(Error::from(err)),
		};

		match result {
			Ok(answer) => {
				let _ = self.status_tx.send(ExecEvent::RunResult(answer)).await;
			}
			Err(err) => {
				let _ = self.status_tx.send(ExecEvent::RunError(err.to_string())).await;
			}
		}

		let _ = self.status_tx.send(ExecEvent::RunEnd).await;

		Ok(())
	}
}
