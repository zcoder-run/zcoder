pub struct AppState {
	input: String,
	waiting: bool,
	status: String,
	last_answer: Option<String>,
	last_error: Option<String>,
}

impl AppState {
	pub fn new(initial_prompt: Option<String>) -> Self {
		Self {
			input: initial_prompt.unwrap_or_default(),
			waiting: false,
			status: "Idle".to_string(),
			last_answer: None,
			last_error: None,
		}
	}

	pub fn input(&self) -> &str {
		&self.input
	}

	pub fn push_input(&mut self, c: char) {
		self.input.push(c);
	}

	pub fn pop_input(&mut self) {
		self.input.pop();
	}

	pub fn clear_input(&mut self) {
		self.input.clear();
	}

	pub fn is_waiting(&self) -> bool {
		self.waiting
	}

	pub fn set_waiting(&mut self, waiting: bool) {
		self.waiting = waiting;
	}

	pub fn status(&self) -> &str {
		&self.status
	}

	pub fn set_status(&mut self, status: String) {
		self.status = status;
	}

	pub fn last_answer(&self) -> Option<&str> {
		self.last_answer.as_deref()
	}

	pub fn set_last_answer(&mut self, answer: Option<String>) {
		self.last_answer = answer;
	}

	pub fn last_error(&self) -> Option<&str> {
		self.last_error.as_deref()
	}

	pub fn set_last_error(&mut self, error: Option<String>) {
		self.last_error = error;
	}
}
