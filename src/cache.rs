use crate::providers::{NuObjectStore, parse_url};
use async_lock::{Mutex, MutexGuard};
use bytes::Bytes;
use nu_plugin::EngineInterface;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::{GetOptions, path::Path};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use url::Url;

pub struct CacheEntry {
    path: Path,
    /// Data returned by last request
    data: Bytes,
    /// ETag identifying the object returned by the server
    e_tag: String,
    /// Instant of last refresh
    refreshed_at: Instant,
    /// Object store used for this file.
    /// todo: ideally there would be a way to reuse this for multiple paths in a generic way
    store: NuObjectStore,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ObjectStoreCacheKey {
    Memory,
    Local,
    AmazonS3 { bucket: String, region: String },
}

impl From<&NuObjectStore> for ObjectStoreCacheKey {
    fn from(value: &NuObjectStore) -> Self {
        match value {
            NuObjectStore::Memory(_) => ObjectStoreCacheKey::Memory,
            NuObjectStore::Local(_) => ObjectStoreCacheKey::Local,
            NuObjectStore::AmazonS3 { bucket, region, .. } => ObjectStoreCacheKey::AmazonS3 {
                bucket: bucket.to_owned(),
                region: region.to_owned(),
            },
            NuObjectStore::GoogleCloudStorage(_) => unimplemented!(),
            NuObjectStore::MicrosoftAzure(_) => unimplemented!(),
            NuObjectStore::Http(_) => unimplemented!(),
        }
    }
}

/// Example cache that checks entries after 10 seconds for a new version
#[derive(Default)]
pub struct Cache {
    entries: Mutex<HashMap<Url, CacheEntry>>,
    stores: Mutex<HashMap<ObjectStoreCacheKey, NuObjectStore>>,
}

impl Cache {
    pub async fn get(
        &self,
        engine: &EngineInterface,
        url: &Spanned<Url>,
        span: Span,
    ) -> Result<Bytes, ShellError> {
        let mut lock = self.entries_cache_lock().await;
        Ok(match lock.get_mut(&url.item) {
            Some(e) => match e.refreshed_at.elapsed() < Duration::from_secs(10) {
                true => e.data.clone(), // Return cached data
                false => {
                    // Check if remote version has changed
                    let opts = GetOptions {
                        if_none_match: Some(e.e_tag.clone()),
                        ..GetOptions::default()
                    };
                    match e.store.object_store().get_opts(&e.path, opts).await {
                        Ok(d) => e.data = d.bytes().await.map_err(cache_get_error)?,
                        Err(object_store::Error::NotModified { .. }) => {} // Data has not changed
                        Err(e) => return Err(cache_get_error(e)),
                    }
                    e.refreshed_at = Instant::now();
                    e.data.clone()
                }
            },
            None => {
                // Not cached, fetch data
                let (store, path) = parse_url(engine, self, url, span).await?;
                let get = store
                    .object_store()
                    .get(&path)
                    .await
                    .map_err(cache_get_error)?;
                let e_tag = get.meta.e_tag.clone();
                let data = get.bytes().await.map_err(cache_get_error)?;
                if let Some(e_tag) = e_tag {
                    let entry = CacheEntry {
                        path,
                        e_tag,
                        data: data.clone(),
                        refreshed_at: Instant::now(),
                        store,
                    };
                    lock.insert(url.item.clone(), entry);
                }
                data
            }
        })
    }

    pub async fn put_store(
        &self,
        engine: &EngineInterface,
        key: ObjectStoreCacheKey,
        store: NuObjectStore,
    ) -> Result<(), ShellError> {
        let mut lock = self.stores_cache_lock().await;
        lock.insert(key, store);
        engine.set_gc_disabled(true)
    }

    pub async fn get_store(&self, key: &ObjectStoreCacheKey) -> Option<NuObjectStore> {
        let lock = self.stores_cache_lock().await;
        lock.get(key).cloned()
    }

    pub async fn clear(&self, engine: &EngineInterface) -> Result<(), ShellError> {
        let mut lock = self.entries_cache_lock().await;
        lock.clear();
        let mut lock = self.stores_cache_lock().await;
        lock.clear();
        engine.set_gc_disabled(false)
    }

    async fn entries_cache_lock(&self) -> MutexGuard<'_, HashMap<Url, CacheEntry>> {
        self.entries.lock().await
    }

    async fn stores_cache_lock(
        &self,
    ) -> MutexGuard<'_, HashMap<ObjectStoreCacheKey, NuObjectStore>> {
        self.stores.lock().await
    }
}

fn cache_get_error(e: impl std::error::Error) -> ShellError {
    ShellError::GenericError {
        error: format!("Error fetching data from obect store: {e}"),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
    }
}
