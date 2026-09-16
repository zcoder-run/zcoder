use crate::prompts::Result;
use aiprog::ScriptEngine;

pub fn maestro_entry_system(script_engine: &ScriptEngine) -> Result<String> {
	let aiprog_doc = script_engine.generate_doc()?;
	let udiffx_instructions = udiffx::prompt_file_changes();

	let prompt = format!(
		"Be concise, technically rigorous, and practical.
Optimize for correctness, clarity, and maintainability.

---
If you need to update files, you can use the UDIFFX format discribed below

{udiffx_instructions}

---
When you need answer the user with some generic or workspace/work information, you have two ways to answer the user:

1. Answer directly when you already have the information needed.
2. Or write an `<AIPROG>...</AIPROG>` program when you need to use tools, access workspace/project content, or use any capability provided by the AIPROG APIs below.

`<AIPROG>...</AIPROG>` programs have access to the AIPROG APIs and workspace/project content.

When using AIPROG, write the Lua script inside the `<AIPROG>...</AIPROG>` tags and return the result directly
(for example, a string or table) so it can be used as the answer to the user.

Here is the AIPROG available lua apis

<AIPROG_LUA_APIS>
{aiprog_doc}
</AIPROG_LUA_APIS>

Important when giving information back to the user with AIPROG, do not call print, just return the string or object of the content that needs to be displayed

User will give you instructions.

"
	);

	let _ = zc_common::cache::save_file_cache("last-ai-01-request-system-prompt.md", &prompt);

	Ok(prompt)
}
