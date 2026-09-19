use crate::model::{WksBmc, get_model_manager};
use futures_util::future::BoxFuture;
use zc_core::model::Id;
use zc_router::{ClientInfo, WksResolver};

pub struct BaseWksResolver;

impl WksResolver for BaseWksResolver {
	fn resolve<'a>(&'a self, info: &'a ClientInfo) -> BoxFuture<'a, zc_router::Result<Id>> {
		Box::pin(async move {
			let mm = get_model_manager().map_err(|e| zc_router::Error::custom(e.to_string()))?;
			let id = WksBmc::get_or_create_by_dir(mm, &info.wspace_dir, info.label.clone())
				.await
				.map_err(|e| zc_router::Error::custom(e.to_string()))?;

			// -- Materialize the workspace `.zcoder/` assets on connect, so a missing
			//    `.zcoder/config.toml` is seeded from the bundled `wspace/config.toml`.
			if let Err(err) = zc_asset::update_wspace_dir(&info.wspace_dir) {
				tracing::warn!("->> failed to sync workspace assets for {}: {err}", info.wspace_dir);
			}

			Ok(id)
		})
	}
}

// region:    --- Tests

#[cfg(test)]
mod tests {
	type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

	use super::*;

	#[tokio::test]
	async fn test_base_wspace_resolver_resolves_dir() -> Result<()> {
		let resolver = BaseWksResolver;
		let info = ClientInfo::from_wspace_dir("/tmp/zc-test-base-wspace-resolver");
		let id1 = resolver.resolve(&info).await?;
		let id2 = resolver.resolve(&info).await?;
		assert_eq!(id1, id2);
		Ok(())
	}

	#[tokio::test]
	async fn test_base_wspace_resolver_materializes_wspace_config() -> Result<()> {
		// -- Setup & Fixtures
		let wspace_dir =
			std::env::temp_dir().join(format!("zc-test-base-wspace-resolver-assets-{}", std::process::id()));
		let wspace_dir_str = wspace_dir.to_string_lossy().to_string();
		let config_path = wspace_dir.join(".zcoder").join("config.toml");
		let _ = std::fs::remove_dir_all(&wspace_dir);

		let resolver = BaseWksResolver;
		let info = ClientInfo::from_wspace_dir(wspace_dir_str.as_str());

		// -- Exec
		resolver.resolve(&info).await?;

		// -- Check
		assert!(
			config_path.exists(),
			"expected .zcoder/config.toml to be materialized on connect"
		);

		// -- Clean
		let _ = std::fs::remove_dir_all(&wspace_dir);
		Ok(())
	}
}

// endregion: --- Tests
