// region:    --- Types

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AiPrice {
	pub cost: f64,
	pub cost_cache_write: Option<f64>,
	pub cost_cache_saving: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPricing {
	pub name: &'static str,
	pub input_cached: Option<f64>,
	pub input_normal: f64,
	pub output_normal: f64,
	pub output_reasoning: Option<f64>,
}

// endregion: --- Types
