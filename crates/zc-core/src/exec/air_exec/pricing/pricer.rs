// region:    --- Modules

use crate::exec::air_exec::pricing::{AiPrice, ModelPricing};
use genai::ModelIden;
use genai::chat::Usage;

// endregion: --- Modules

// region:    --- Public Functions

/// Computes the AI price for a given provider, model, and token usage using `aicost`.
pub fn price_it(provider_type: &str, model_name: &str, usage: &Usage) -> Option<AiPrice> {
	let ai_cost = aicost::compute(provider_type, model_name, usage)
		.or_else(|_| aicost::compute(&provider_type.to_lowercase(), model_name, usage))
		.ok()?;
	Some(AiPrice {
		cost: ai_cost.total,
		cost_cache_write: (ai_cost.input_cache_write > 0.0).then_some(ai_cost.input_cache_write),
		cost_cache_saving: (ai_cost.input_cache_saving != 0.0).then_some(ai_cost.input_cache_saving),
	})
}

/// Retrieves pricing metadata for a specified model identifier.
pub fn model_pricing(model_iden: &ModelIden) -> Option<ModelPricing> {
	let pricing = aicost::model_pricing(model_iden)?;
	Some(ModelPricing {
		name: pricing.name,
		input_cached: pricing.input_cached,
		input_normal: pricing.input_normal,
		output_normal: pricing.output_normal,
		output_reasoning: pricing.output_reasoning,
	})
}

// endregion: --- Public Functions

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;
	use genai::adapter::AdapterKind;
	use genai::chat::Usage;

	#[test]
	fn test_air_exec_pricing_price_it_simple() -> Result<()> {
		// -- Setup & Fixtures
		let usage = Usage {
			prompt_tokens: Some(1000),
			completion_tokens: Some(500),
			total_tokens: Some(1500),
			prompt_tokens_details: None,
			completion_tokens_details: None,
		};

		// -- Exec
		let price = price_it("openai", "gpt-4o", &usage).ok_or("should compute price for gpt-4o")?;

		// -- Check
		assert!(price.cost > 0.0);
		// Also verify case-insensitivity (e.g. from AdapterKind::as_str() returning "OpenAI")
		let price_upper = price_it("OpenAI", "gpt-4o", &usage);
		assert!(price_upper.is_some(), "pricing for OpenAI should be found");
		assert_eq!(price.cost, price_upper.ok_or("should have price")?.cost);

		Ok(())
	}

	#[test]
	fn test_air_exec_pricing_model_pricing_lookup() -> Result<()> {
		// -- Setup & Fixtures
		let model_iden = ModelIden::from((AdapterKind::OpenAI, "gpt-4o"));

		// -- Exec
		let pricing = model_pricing(&model_iden).ok_or("should find model pricing for gpt-4o")?;

		// -- Check
		assert!(pricing.input_normal > 0.0);
		assert!(pricing.output_normal > 0.0);

		Ok(())
	}
}

// endregion: --- Tests
