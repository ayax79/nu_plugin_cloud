use super::NuObjectStore;
use crate::cache::{Cache, ObjectStoreCacheKey};
use object_store::memory::InMemory;
use std::sync::Arc;

pub async fn build_object_store(cache: &Cache) -> NuObjectStore {
    let key = ObjectStoreCacheKey::Memory;
    if let Some(store) = cache.get_store(&key).await {
        store
    } else {
        let store = InMemory::new();
        let store = NuObjectStore::Memory(Arc::new(store));
        cache.put_store(key, store.clone()).await;
        store
    }
}
