use super::NuObjectStore;
use crate::cache::{Cache, ObjectStoreCacheKey};
use nu_plugin::EngineInterface;
use nu_protocol::ShellError;
use object_store::memory::InMemory;
use std::sync::Arc;

pub async fn build_object_store(
    engine: &EngineInterface,
    cache: &Cache,
) -> Result<NuObjectStore, ShellError> {
    let key = ObjectStoreCacheKey::Memory;
    if let Some(store) = cache.get_store(&key).await {
        Ok(store)
    } else {
        let store = InMemory::new();
        let store = NuObjectStore::Memory(Arc::new(store));
        cache.put_store(engine, key, store.clone()).await?;
        Ok(store)
    }
}
