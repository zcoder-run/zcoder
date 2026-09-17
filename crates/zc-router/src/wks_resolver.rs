use crate::client_info::ClientInfo;
use crate::error::Result;
use futures_util::future::BoxFuture;
use zc_core::model::Id;

// region:    --- WksResolver

/// Seam that resolves a client's workspace directory to the workspace id the base owns.
///
/// This mirrors the `RequestHandler` seam in the rust10x `ipc-socket-design`
/// sample: the transport owns the mechanics, the application owns the meaning.
/// `zc-router` turns a `wks_dir` into a `wks_id` without depending on the model
/// layer. The future is boxed so the trait stays object-safe and the server can
/// hold an `Arc<dyn WksResolver>`.
pub trait WksResolver: Send + Sync + 'static {
	/// Returns the workspace id for the given client info, creating it if missing.
	fn resolve<'a>(&'a self, info: &'a ClientInfo) -> BoxFuture<'a, Result<Id>>;
}

// endregion: --- WksResolver

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	struct StubResolver;

	impl WksResolver for StubResolver {
		fn resolve<'a>(&'a self, _info: &'a ClientInfo) -> BoxFuture<'a, crate::error::Result<Id>> {
			Box::pin(async { Ok(Id::default()) })
		}
	}

	#[tokio::test]
	async fn test_wks_resolver_stub_is_object_safe() -> Result<()> {
		// -- Setup & Fixtures
		let info = ClientInfo::from_wks_dir("/home/dev/zcoder");
		let resolver: std::sync::Arc<dyn WksResolver> = std::sync::Arc::new(StubResolver);

		// -- Exec
		let wks_id = resolver.resolve(&info).await?;

		// -- Check
		assert_eq!(wks_id, Id::default());

		Ok(())
	}
}

// endregion: --- Tests
