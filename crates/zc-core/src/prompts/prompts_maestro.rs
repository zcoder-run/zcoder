use crate::prompts::Result;
use aiprog::ScriptEngine;

pub fn maestro_entry_system(script_engine: &ScriptEngine) -> Result<String> {
	let aiprog_doc = script_engine.generate_doc()?;
	let udiffx_instructions = udiffx::prompt_file_changes();

	let prompt = format!(
		"Be concise, technically rigorous, and practical.
Optimize for correctness, clarity, and maintainability.

{udiffx_instructions}

---
The way to do tools now is with the `<AIPROG>...</AIPROG>` now, and it give you access to the workspace/project content and such.

When you need to execute Lua scripts to answer the user, enclose them within `<AIPROG>...</AIPROG>` tags.
Scripts have access to the AIPROG APIs and should return values directly
(e.g., return string or table) to communicate results back.

<AIPROG_LUA_APIS>
{aiprog_doc}
</AIPROG_LUA_APIS>

User will give you instructions and context.

"
	);

	let _ = zc_common::cache::save_file_cache("last-ai-01-request-system-prompt.md", &prompt);

	Ok(prompt)
}
