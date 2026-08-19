use crate::exec::{Error, ExecCmd, ExecCmdRx, ExecCmdTx, ExecEvent, ExecEventRx, ExecEventTx, Result};
use crate::model::{ModelManager, RunBmc, RunForCreate, RunForUpdate};
use genai::chat::{ChatMessage, ChatRequest};
use zc_common::event_base::new_mpsc_bounded;

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
	script_engine: aiprog::ScriptEngine,
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
	pub fn new(config: ExecutorConfig) -> Result<(Self, ExecCmdTx, ExecEventRx)> {
		let (action_tx, action_rx) = new_mpsc_bounded::<ExecCmd>("executor_channel", 1000)?;
		let (status_tx, status_rx) = new_mpsc_bounded::<ExecEvent>("executor_channel", 1000)?;

		let aip_registry = aiprog::AipRegistry::from_aip_modules()?;
		let script_engine = aiprog::ScriptEngine::builder().with_registry(aip_registry).build()?;
		let aiprog_doc = script_engine.generate_doc()?;

		let base_chat_req = ChatRequest::from_system(format!(
			"You are a senior developer. User will give you instructions and context.\n\n{}\n\nWhen you need to execute Lua scripts, enclose them within `<AIPROG>...</AIPROG>` tags. Scripts have access to the AIPROG APIs and should return values directly (e.g., return string or table) to communicate results back.\n\n<AIPROG_LUA_APIS>\n{}\n</AIPROG_LUA_APIS>",
			udiffx::prompt_file_changes(),
			aiprog_doc
		));

		Ok((
			Self {
				action_rx,
				inner: ExecutorInner {
					status_tx,
					genai_client: genai::Client::default(),
					base_chat_req,
					base_dir: config.base_dir,
					model: config.model,
					src_globs: config.src_globs,
					script_engine,
				},
			},
			action_tx,
			status_rx,
		))
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
		let run_id = RunBmc::create(mm, run_c).await?;

		// -- Prep clones for the async block to avoid moving `self`
		let status_tx = self.status_tx.clone();
		let mut chat_req = self.base_chat_req.clone();
		let base_dir = self.base_dir.clone(); // Assumes PathBuf or String that can clone
		let src_globs = self.src_globs;
		let genai_client = self.genai_client.clone(); // Assumes your client is cheaply cloneable (Arc-backed)
		let model = self.model; // Assumes Copy/Clone (like &str or Copy enum)
		let script_engine = self.script_engine.clone();

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

			let _ = zc_common::cache::save_file_cache("last-ai-response-raw.md", &ai_response);

			// -- Process AI Response
			let (file_changes, other_content) = udiffx::extract_file_changes(&ai_response, true)?;
			let _change_statuses = udiffx::apply_file_changes(&base_dir, file_changes)?;
			let raw_answer = other_content.unwrap_or_default();

			// -- Process and execute AIPROG scripts if present
			let (aiprog_scripts, mut answer) = extract_aiprog_scripts(&raw_answer);

			for lua_script in aiprog_scripts {
				let mut running_context = aiprog::RunningContext::default();
				if let Ok(dir_ctx) = create_dir_context(&base_dir) {
					running_context.insert(dir_ctx);
				}

				let script_engine_clone = script_engine.clone();
				let outcome_result: core::result::Result<String, String> =
					tokio::task::spawn_blocking(move || -> Result<core::result::Result<String, String>> {
						let rt = tokio::runtime::Builder::new_current_thread()
							.enable_all()
							.build()
							.map_err(|e| Error::custom(format!("Failed to create tokio runtime for Lua: {e}")))?;
						let outcome = rt
							.block_on(script_engine_clone.exec(&lua_script, running_context))
							.map_err(|e| Error::Aiprog(e.to_string()))?;
						Ok(outcome.result.map(|val| format!("{val:#?}")).map_err(|err| err.to_string()))
					})
					.await
					.map_err(|e| Error::custom(format!("Lua execution join error: {e}")))??;

				match outcome_result {
					Ok(val_str) => {
						if !answer.trim().is_empty() {
							answer.push_str("\n\n");
						}
						answer.push_str(&format!("### Lua Execution Result\n\n```\n{val_str}\n```"));
					}
					Err(err_str) => {
						if !answer.trim().is_empty() {
							answer.push_str("\n\n");
						}
						answer.push_str(&format!("### Lua Execution Error\n\n```\n{err_str}\n```"));
					}
				}
			}

			// -- Store response
			RunBmc::update(
				mm,
				run_id,
				RunForUpdate {
					answer: Some(answer.clone()),
					..Default::default()
				},
			)
			.await?;

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
			)
			.await?;

			let _ = self.status_tx.send(ExecEvent::RunError(run_id)).await;

			// Optionally return the error or return Ok(()) depending on requirements
			return Err(err);
		}

		Ok(())
	}
}

// region:    --- Support

fn create_dir_context(base_dir: &str) -> core::result::Result<aiprog::DirContext, aiprog::DirPolicyError> {
	let _ = simple_fs::ensure_dir(base_dir);
	let read_policy = aiprog::PathPolicy::new([base_dir], aiprog::AbsolutePathPolicy::Allow)?;
	let write_policy = aiprog::PathPolicy::new([base_dir], aiprog::AbsolutePathPolicy::Allow)?;
	Ok(aiprog::DirContext::new(read_policy, write_policy))
}

fn extract_aiprog_scripts(content: &str) -> (Vec<String>, String) {
	let parts = markex::tag::extract(content, &["AIPROG"], true);
	let mut scripts = Vec::new();
	let mut text_parts = Vec::new();

	for part in parts.parts() {
		match part {
			markex::tag::Part::TagElem(elem) => {
				let trimmed = elem.content.trim();
				if !trimmed.is_empty() {
					scripts.push(trimmed.to_string());
				}
			}
			markex::tag::Part::Text(txt) => {
				text_parts.push(txt.as_str());
			}
		}
	}

	(scripts, text_parts.join("").trim().to_string())
}

// endregion: --- Support

// region:    --- Tests

#[cfg(test)]
mod tests {
	use super::*;
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	#[test]
	fn test_exec_extract_aiprog_scripts_simple() -> Result<()> {
		// -- Setup & Fixtures
		let content = "Here is the plan:\n<AIPROG>\nlocal x = 1 + 2\nreturn x\n</AIPROG>\nDone.";

		// -- Exec
		let (scripts, text) = extract_aiprog_scripts(content);

		// -- Check
		let first_script = scripts.first().ok_or("Should have one script")?;
		assert_eq!(first_script, "local x = 1 + 2\nreturn x");
		assert_eq!(text, "Here is the plan:\n\nDone.");
		Ok(())
	}

	#[test]
	fn test_exec_extract_aiprog_scripts_none() -> Result<()> {
		// -- Setup & Fixtures
		let content = "No aiprog tag here.";

		// -- Exec
		let (scripts, text) = extract_aiprog_scripts(content);

		// -- Check
		assert!(scripts.is_empty());
		assert_eq!(text, "No aiprog tag here.");
		Ok(())
	}
}

// endregion: --- Tests
