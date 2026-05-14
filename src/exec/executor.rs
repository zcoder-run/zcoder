use crate::exec::{ExecActionEvent, ExecStatusEvent};
use crate::{Error, Result};
use genai::chat::{ChatMessage, ChatRequest};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct Executor {
	action_rx: Receiver<ExecActionEvent>,
	status_tx: Sender<ExecStatusEvent>,
	// State needed for execution
	genai_client: genai::Client,
	base_chat_req: ChatRequest,
	base_dir: String,
	model: &'static str,
	src_globs: &'static [&'static str],
}

#[derive(Clone)]
pub struct ExecutorTx {
	tx: Sender<ExecActionEvent>,
}

impl ExecutorTx {
	pub async fn send(&self, action: ExecActionEvent) -> Result<()> {
		self.tx
			.send(action)
			.await
			.map_err(|_| Error::custom("Executor channel closed"))
	}
}

impl Executor {
	pub fn new(
		status_tx: Sender<ExecStatusEvent>,
		genai_client: genai::Client,
		base_chat_req: ChatRequest,
		base_dir: String,
		model: &'static str,
		src_globs: &'static [&'static str],
	) -> (Self, ExecutorTx) {
		let (tx, rx) = mpsc::channel(100);
		(
			Self {
				action_rx: rx,
				status_tx,
				genai_client,
				base_chat_req,
				base_dir,
				model,
				src_globs,
			},
			ExecutorTx { tx },
		)
	}

	pub async fn start(mut self) {
		while let Some(action) = self.action_rx.recv().await {
			match action {
				ExecActionEvent::RunPrompt(prompt) => {
					let _ = self.handle_run_prompt(prompt).await;
				}
			}
		}
	}

	async fn handle_run_prompt(&self, prompt: String) -> Result<()> {
		let _ = self.status_tx.send(ExecStatusEvent::RunStart).await;

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
				let ai_response = res.content.into_first_text().ok_or_else(|| Error::custom("Should have response"))?;

				// -- Process AI Response
				let (file_changes, other_content) = udiffx::extract_file_changes(&ai_response, true)?;
				let _change_statuses = udiffx::apply_file_changes(&self.base_dir, file_changes)?;

				Ok(other_content.unwrap_or_default())
			}
			Err(err) => Err(Error::from(err)),
		};

		match result {
			Ok(answer) => {
				let _ = self.status_tx.send(ExecStatusEvent::RunResult(answer)).await;
			}
			Err(err) => {
				let _ = self.status_tx.send(ExecStatusEvent::RunError(err.to_string())).await;
			}
		}

		let _ = self.status_tx.send(ExecStatusEvent::RunEnd).await;

		Ok(())
	}
}
