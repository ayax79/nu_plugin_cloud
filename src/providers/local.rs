use super::NuObjectStore;
use crate::cache::{Cache, ObjectStoreCacheKey};
use nu_plugin::EngineInterface;
use nu_protocol::ShellError;
use object_store::local::LocalFileSystem;
use std::sync::Arc;

pub async fn build_object_store(
    engine: &EngineInterface,
    cache: &Cache,
) -> Result<NuObjectStore, ShellError> {
    let key = ObjectStoreCacheKey::Local;
    if let Some(store) = cache.get_store(&key).await {
        Ok(store)
    } else {
        let store = LocalFileSystem::new();
        let store = NuObjectStore::Local(Arc::new(store));
        cache.put_store(engine, key, store.clone()).await?;
        Ok(store)
    }
}
