use crate::exec::{Error, ExecCmd, ExecCmdRx, ExecCmdTx, ExecEvent, ExecEventRx, ExecEventTx, Result, exec_air_chat};
use crate::model::{EpochUs, ModelManager, RunBmc, RunEndState, RunForCreate, RunForUpdate};
use crate::prompts;
use genai::chat::{ChatMessage, ChatRequest};
use zc_common::event_base::new_mpsc_bounded;

// -- Consts (harcoded for now)
const DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

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
	script_engine: aiprog::ScriptEngine,
}

#[derive(Debug, Clone)]
pub struct ExecutorConfig {
	base_dir: String,
	model: &'static str,
}

impl Default for ExecutorConfig {
	fn default() -> Self {
		Self {
			base_dir: ".demo-dir/".to_string(),
			model: DEFAULT_MODEL,
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
}

impl Executor {
	pub fn new(config: ExecutorConfig) -> Result<(Self, ExecCmdTx, ExecEventRx)> {
		let (action_tx, action_rx) = new_mpsc_bounded::<ExecCmd>("executor_channel", 1000)?;
		let (status_tx, status_rx) = new_mpsc_bounded::<ExecEvent>("executor_channel", 1000)?;

		let aip_registry = aiprog::AipRegistry::from_aip_modules()?;
		let script_engine = aiprog::ScriptEngine::builder().with_registry(aip_registry).build()?;

		// -- Build the base ai request
		let system_prompt = prompts::maestro_entry_system(&script_engine)?;
		let base_chat_req = ChatRequest::from_system(system_prompt);

		Ok((
			Self {
				action_rx,
				inner: ExecutorInner {
					status_tx,
					genai_client: genai::Client::default(),
					base_chat_req,
					base_dir: config.base_dir,
					model: config.model,
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
	async fn handle_run_prompt(&self, mm: &'static ModelManager, prompt: String) -> Result<()> {
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
		let genai_client = self.genai_client.clone(); // Assumes your client is cheaply cloneable (Arc-backed)
		let model = self.model; // Assumes Copy/Clone (like &str or Copy enum)
		let script_engine = self.script_engine.clone();

		// Use an async block with an explicit type annotation
		let block_result: Result<()> = async move {
			// -- Send RunStart
			let _ = status_tx.send(ExecEvent::RunStart(run_id)).await;

			// -- TODO: Load previous context from prog

			// -- Build Prompt / Context
			chat_req = chat_req.append_message(ChatMessage::user(prompt));

			// -- Execute Air Request
			let (res, _air_id) = exec_air_chat(mm, &genai_client, model, chat_req, run_id, None).await?;

			let ai_response = res
				.content
				.into_first_text()
				.ok_or_else(|| Error::custom("Should have response"))?;

			let _ = zc_common::cache::save_file_cache("last-ai-response-raw.md", &ai_response);

			// -- Process UDIFFX
			let (file_changes, other_content) = udiffx::extract_file_changes(&ai_response, true)?;
			let _change_statuses = udiffx::apply_file_changes(&base_dir, file_changes)?;
			let raw_answer = other_content.unwrap_or_default();

			// -- Process AIPROG
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
							.map_err(Error::from)?;
						Ok(outcome
							.result
							.map(format_lua_outcome_value)
							.map_err(|err| format_lua_outcome_error(&err)))
					})
					.await
					.map_err(|e| Error::custom(format!("Lua execution join error: {e}")))??;

				match outcome_result {
					Ok(val_str) => {
						if !answer.trim().is_empty() {
							answer.push_str("\n\n");
						}
						answer.push_str(&val_str);
					}
					Err(err_str) => {
						if !answer.trim().is_empty() {
							answer.push_str("\n\n");
						}
						answer.push_str(&err_str);
					}
				}
			}

			// -- Store response
			let end_time = EpochUs::now();
			RunBmc::update(
				mm,
				run_id,
				RunForUpdate {
					answer: Some(answer.clone()),
					end: Some(end_time),
					end_state: Some(RunEndState::Success.to_string()),
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
			let end_time = EpochUs::now();
			RunBmc::update(
				mm,
				run_id,
				RunForUpdate {
					error: Some(err.to_string()),
					end: Some(end_time),
					end_state: Some(RunEndState::Error.to_string()),
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

fn create_dir_context(base_dir: &str) -> Result<aiprog::DirContext> {
	let _ = simple_fs::ensure_dir(base_dir);
	(|| -> core::result::Result<_, _> {
		let read_policy = aiprog::PathPolicy::new([base_dir], aiprog::AbsolutePathPolicy::Allow)?;
		let write_policy = aiprog::PathPolicy::new([base_dir], aiprog::AbsolutePathPolicy::Allow)?;
		aiprog::DirContext::new(base_dir, read_policy, write_policy)
	})()
	.map_err(super::Error::custom_from_err)
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

fn format_lua_outcome_value(val: serde_json::Value) -> String {
	match val {
		serde_json::Value::String(s) => s,
		other => zc_common::yaml::json_to_yaml_string(&other).unwrap_or_else(|_| other.to_string()),
	}
}

fn format_lua_outcome_error(err: &aiprog::Error) -> String {
	super::Error::from(err).to_string()
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

	#[test]
	fn test_exec_format_lua_outcome_value_string() -> Result<()> {
		// -- Setup & Fixtures
		let val = serde_json::Value::String("hello world\nsecond line".to_string());

		// -- Exec
		let formatted = format_lua_outcome_value(val);

		// -- Check
		assert_eq!(formatted, "hello world\nsecond line");
		Ok(())
	}

	#[test]
	fn test_exec_format_lua_outcome_value_object() -> Result<()> {
		// -- Setup & Fixtures
		let val = serde_json::json!({
			"status": "ok",
			"code": 200
		});

		// -- Exec
		let formatted = format_lua_outcome_value(val);

		// -- Check
		assert!(formatted.contains("status: ok"));
		assert!(formatted.contains("code: 200"));
		Ok(())
	}

	#[test]
	fn test_exec_format_lua_outcome_error_with_surround() -> Result<()> {
		// -- Setup & Fixtures
		let script = "local x = 1\nlocal y = undefined_function()\nreturn y";
		let details = aiprog::LuaErrorDetails::new(
			std::sync::Arc::from(script),
			Some(2),
			"attempt to call a nil value (global 'undefined_function')",
			None,
		);
		let err = aiprog::Error::LuaScript(details);

		// -- Exec
		let formatted = format_lua_outcome_error(&err);

		// -- Check
		assert!(formatted.contains("Lua Error (line 2): attempt to call a nil value"));
		assert!(formatted.contains("```lua"));
		Ok(())
	}

	#[test]
	fn test_exec_format_lua_outcome_error_simple() -> Result<()> {
		// -- Setup & Fixtures
		let details =
			aiprog::LuaErrorDetails::new(std::sync::Arc::from("test"), None, "global variable not found", None);
		let err = aiprog::Error::LuaScript(details);

		// -- Exec
		let formatted = format_lua_outcome_error(&err);

		// -- Check
		assert_eq!(formatted, "Lua Error: global variable not found");
		Ok(())
	}

	#[test]
	fn test_exec_format_lua_outcome_error_engine_custom() -> Result<()> {
		// -- Setup & Fixtures
		let raw_msg = "runtime error: script:5: attempt to index a number value\nstack traceback:\n\tscript:5: in main";
		let err = aiprog::Error::Engine(aiprog::EngineError::Custom(raw_msg.to_string()));

		// -- Exec
		let formatted = format_lua_outcome_error(&err);

		// -- Check
		assert_eq!(formatted, raw_msg);
		Ok(())
	}
}

// endregion: --- Tests
