use crate::model::{WksBmc, get_model_manager};
use futures_util::future::BoxFuture;
use zc_core::model::Id;
use zc_router::{ClientInfo, WksResolver};

pub struct BaseWksResolver;

impl WksResolver for BaseWksResolver {
	fn resolve<'a>(&'a self, info: &'a ClientInfo) -> BoxFuture<'a, zc_router::Result<Id>> {
		Box::pin(async move {
			let mm = get_model_manager().map_err(|e| zc_router::Error::custom(e.to_string()))?;
			let id = WksBmc::get_or_create_by_dir(mm, &info.wks_dir, info.label.clone())
				.await
				.map_err(|e| zc_router::Error::custom(e.to_string()))?;
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
	async fn test_base_wks_resolver_resolves_dir() -> Result<()> {
		let resolver = BaseWksResolver;
		let info = ClientInfo::from_wks_dir("/tmp/zc-test-base-wks-resolver");
		let id1 = resolver.resolve(&info).await?;
		let id2 = resolver.resolve(&info).await?;
		assert_eq!(id1, id2);
		Ok(())
	}
}

// endregion: --- Tests
