use crate::model::ModelEvent;
use std::sync::OnceLock;
use tokio::sync::broadcast;

const DEFAULT_MODEL_BUS_CAPACITY: usize = 1024;

static MODEL_BUS: OnceLock<ModelBus> = OnceLock::new();
pub fn get_model_bus() -> &'static ModelBus {
	MODEL_BUS.get_or_init(ModelBus::new)
}

#[derive(Debug)]
pub struct ModelBus {
	_rx: broadcast::Receiver<ModelEvent>,
	tx: broadcast::Sender<ModelEvent>,
}

impl ModelBus {
	fn new() -> Self {
		let (tx, _rx) = broadcast::channel(DEFAULT_MODEL_BUS_CAPACITY);
		Self { tx, _rx }
	}
}

impl ModelBus {
	pub fn subscribe(&self) -> broadcast::Receiver<ModelEvent> {
		self.tx.subscribe()
	}

	pub fn publish(&self, event: ModelEvent) -> usize {
		self.tx.send(event).unwrap_or(0)
	}

	pub fn receiver_count(&self) -> usize {
		self.tx.receiver_count()
	}
}
