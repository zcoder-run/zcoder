use crate::exec::{Error, ExecCmd, ExecCmdRx, ExecCmdTx, ExecEvent, ExecEventRx, ExecEventTx, Result};
use crate::model::{ModelManager, RunBmc, RunForCreate, RunForUpdate};
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
	action_rx: ExecCmdRx,
	inner: ExecutorInner,
}

struct ExecutorInner {
	status_tx: ExecEventTx,
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
	pub fn new(config: ExecutorConfig) -> (Self, ExecCmdTx, ExecEventRx) {
		let (action_tx, action_rx) = new_mpsc_bounded::<ExecCmd>();
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

		let mm = crate::model::get_model_manager()?;

		while let Ok(action) = action_rx.recv().await {
			match action {
				ExecCmd::RunPrompt(prompt) => {
					let _ = inner.handle_run_prompt(mm, prompt).await;
				}
			}
		}

		Ok(())
	}
}

impl ExecutorInner {
	async fn handle_run_prompt(&self, mm: &ModelManager, prompt: String) -> Result<()> {
		// -- Create in the DB
		let run_c = RunForCreate {
			prompt: Some(prompt.clone()),
			answer: None,
		};
		let run_id = RunBmc::create(mm, run_c)?;

		// -- Prep clones for the async block to avoid moving `self`
		let status_tx = self.status_tx.clone();
		let mut chat_req = self.base_chat_req.clone();
		let base_dir = self.base_dir.clone(); // Assumes PathBuf or String that can clone
		let src_globs = self.src_globs;
		let genai_client = self.genai_client.clone(); // Assumes your client is cheaply cloneable (Arc-backed)
		let model = self.model; // Assumes Copy/Clone (like &str or Copy enum)

		// Use an async block with an explicit type annotation
		let block_result: Result<()> = async move {
			// -- Send RunStart
			let _ = status_tx.send(ExecEvent::RunStart(run_id)).await;

			// -- Exec AI
			chat_req = chat_req.append_message(ChatMessage::user(prompt));

			// load file context
			let files_context = udiffx::load_files_context(&base_dir, src_globs)?;
			if let Some(files_context) = files_context {
				chat_req = chat_req.append_message(ChatMessage::user(files_context));
			}

			// execute chat
			let res = genai_client.exec_chat(model, chat_req, None).await?;

			let ai_response = res
				.content
				.into_first_text()
				.ok_or_else(|| Error::custom("Should have response"))?;

			// -- Process AI Response
			let (file_changes, other_content) = udiffx::extract_file_changes(&ai_response, true)?;
			let _change_statuses = udiffx::apply_file_changes(&base_dir, file_changes)?;
			let answer = other_content.unwrap_or_default();

			// -- Store response
			RunBmc::update(
				mm,
				run_id,
				RunForUpdate {
					answer: Some(answer.clone()),
					..Default::default()
				},
			)?;

			// -- send the status event
			let _ = status_tx.send(ExecEvent::RunEnd(run_id)).await;

			Ok(()) // Explicitly return Ok from the async block
		}
		.await;

		// -- Handle error using your TODO pattern
		if let Err(err) = block_result {
			RunBmc::update(
				mm,
				run_id,
				RunForUpdate {
					error: Some(err.to_string()),
					..Default::default()
				},
			);

			let _ = self.status_tx.send(ExecEvent::RunError(run_id)).await;

			// Optionally return the error or return Ok(()) depending on requirements
			return Err(err);
		}

		Ok(())
	}
}
