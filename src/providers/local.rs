use super::NuObjectStore;
use crate::cache::{Cache, ObjectStoreCacheKey};
use object_store::local::LocalFileSystem;
use std::sync::Arc;

pub async fn build_object_store(cache: &Cache) -> NuObjectStore {
    let key = ObjectStoreCacheKey::Local;
    if let Some(store) = cache.get_store(&key).await {
        store
    } else {
        let store = LocalFileSystem::new();
        let store = NuObjectStore::Local(Arc::new(store));
        cache.put_store(key, store.clone()).await;
        store
    }
}
